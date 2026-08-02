# Chapter 6 — Three Thousand Lines of `main.rs`

> **The question this chapter answers:** What did the API layer accumulate, which
> parts genuinely belong there, and which extractions would actually pay?

---

## 1. The number everyone reacts to is the wrong number

`crates/api/src/main.rs` is 3,028 lines. That figure has been in every summary of
this project, including mine, and it is the first thing a reviewer will reach for.

`#[cfg(test)]` starts at line **2,083**.

```
total          3,028
production     2,082
tests            947   (31% of the file)
```

So the honest number is **2,082 lines of production code**, with a test module
holding 59 tests and 171 assertions living in the same file — which is idiomatic
Rust, not sprawl. Unit tests go next to the code in this language. Counting them
toward a "3,000-line file" complaint is counting the wrong thing.

2,082 is still large. It is not remarkable.

## 2. It is not one program. It is two.

Walking the file top to bottom and measuring each region:

| lines | region |
|---:|---|
| 232 | setup — HTTP client, `LlmConfig`, rate limiter, `AppState` |
| 73 | router, CORS, error helper |
| 173 | `card_for` + the plain REST handlers |
| 213 | address search (curated-first) |
| 159 | OpenRouter plumbing + grounding block |
| 431 | agent — prompts, law search, legal directory, ranking |
| 432 | agent — tool schemas + dispatch |
| 236 | agent — chat handler |
| 133 | citations, summary handler, `main` |

Group those:

```
plain REST API      824 lines
LLM agent + LLM     1,258 lines   (60% of production)
```

**The agent is bigger than the product it wraps.** Nine routes are registered;
seven of them are ordinary JSON-over-SQLite handlers that never touch the network,
and they account for 824 lines. The other two — `/summary` and `/agent/chat` —
plus everything they need, account for 1,258.

That reframes the whole complaint. This is not a file that grew unmanageable
serving one purpose. It is two programs with very different shapes — one
synchronous, local, deterministic, pure SQLite; the other async, networked,
non-deterministic, spending money — sharing a compilation unit because they were
written in the same sitting.

**And the seam between them is already clean.** Every call `dispatch_tool` makes
into the rest of the system:

```
card_for   search_curated   rank_by_priorities   scoring::rent_fairness
get_building   get_open_violations   get_tract_median   geosearch_lookup
```

The agent is a *consumer* of the core, not tangled through it. There is no place
where a REST handler calls into agent machinery. The dependency arrow points one
way already, which is the expensive part of an extraction and it is done.

## 3. What this file gets right

Chapters 2 through 5 found real defects, so it matters to be equally precise when
the code is good. Four things here are better than the median production service.

**The lock discipline is compiler-enforced, not documented.** `AppState` holds
`Arc<Mutex<rusqlite::Connection>>` using `std::sync::Mutex` (`:8`, `:189`). Its
guard is `!Send`. Holding one across an `.await` makes the enclosing future `!Send`,
and axum's `Handler` bound requires `Send` — so the code does not compile. Every
handler that calls OpenRouter therefore *cannot* hold the database lock across the
network call, whether or not the author remembered. The comments say "drop the
guard before any await"; the type system is what actually guarantees it.

**The spend guard is symmetric across both paid paths.** Both `/agent/chat`
(`:1801`) and `/summary` (`:2011`) resolve `client_key(&headers)` and check the
same `RateLimiter`. Both place the check *after* the 404 and 501 branches, with the
reason written down:

> Placed after the 404/501 checks so probing costs the caller no quota.

That ordering is a real design decision — probe traffic for unknown buildings or a
key-less deployment shouldn't consume a real user's allowance — and it is applied
identically in two places written at different times.

**The rate limiter is honest about what it is.** From `client_key` (`:176-180`):

> Both are client-supplied in principle, so this is a spend guard, not an
> authentication boundary — a determined attacker can rotate the header. It stops
> casual abuse and honest runaway loops, which is what it is for.

That is the correct characterisation, stated at the definition, and it prevents the
most common failure around IP-keyed limiters: someone downstream mistaking one for
access control.

**The rejected dependency is documented with its reason.** In `app_with_state`:

> we evaluated `tower_governor` 0.8 … its per-client `PeerIpKeyExtractor` needs
> `ConnectInfo<SocketAddr>` … which the `axum-test` mock transport used by this
> crate's test suite does not populate, so it would 500 every test.

A rejected-alternatives note at the point of the compromise. The next person to ask
"why isn't this using a real rate-limiting crate?" gets the answer without
re-deriving it. Note also what it implies: `ConcurrencyLimitLayer::new(64)` bounds
*in-flight* requests, not requests per client per minute — it is not a rate
limiter, and the cost paths are guarded by the hand-rolled one instead. Reading the
comment carefully tells you exactly that.

## 4. Where the weight actually sits

Three functions hold 604 lines — **29% of production code in three items**:

```
238   async fn dispatch_tool          :1452
205   async fn agent_chat_handler     :1745
161   fn tool_schemas                 :1291
```

All three are agent. The largest non-agent function in the file is
`search_handler` at 94 lines, and the median top-level item is well under 40.

`tool_schemas` is the one worth looking at, because it is Chapter 5's defect moved
up a layer:

```rust
fn tool_schemas() -> serde_json::Value {
```

161 lines of hand-built `serde_json::json!` describing eight tools — names,
descriptions, parameter types, required fields. It is a schema, expressed as an
untyped value, with no Rust type corresponding to any of it. `dispatch_tool` then
matches on those same eight names as string literals. Two hand-maintained lists
that must agree, and neither can see the other.

## 5. The list that nothing checks

There are actually **three** copies of the tool list:

1. the eight `"name"` fields in `tool_schemas` (`:1291-1451`)
2. the eight `match` arms in `dispatch_tool` (`:1452-1690`)
3. eight string literals hardcoded inside the test

The test is real and it is not weak:

```rust
for expected in ["get_building", "get_open_violations", ... ] {
    assert!(names.contains(&expected), "missing tool: {expected}");
}
assert_eq!(arr.len(), 8);
```

The `assert_eq!(arr.len(), 8)` is the good part — adding a ninth tool to the schema
fails the test until someone updates the list deliberately. That closes list 1
against list 3.

**Nothing closes list 1 against list 2.** No test asserts that every advertised tool
name reaches a real dispatch arm. Per-tool coverage is genuinely good — all eight
names appear in the test module, across twelve `dispatch_tool` call sites — but
that is eight individual tests, not a set-equality check.

The failure mode is mild, which is why it is worth naming precisely rather than
dramatising. Advertise a tool with no dispatch arm and you hit the fallback, which
is itself tested (`unknown_tool_name_is_reported_back_not_fatal`): the model calls
the tool, gets "unknown tool" back as data, and apologises to the user. No crash,
no 500 — a capability the system claims to have and silently does not. The reverse,
a dispatch arm with no schema entry, is dead code the model can never invoke.

One test closes it:

```rust
for name in schema_names {
    let out = dispatch_tool(&state, "test-key", name, &json!({})).await;
    assert!(!out.to_string().contains("unknown tool"), "undispatched: {name}");
}
```

Given the existing fixtures, that is a few lines, and it converts a hand-maintained
correspondence into a checked one.

## 6. The extraction that pays, and the one that doesn't

**Worth doing: `crates/agent`.** Move the 1,258 agent lines into their own crate
depending on `model`, `scoring`, `store`. The dependency arrow already points that
way, so this is mechanical rather than a redesign. What it buys is not tidiness:

- The agent is the only part of the system that is non-deterministic, networked, and
  spends money. Chapter 2's whole argument is that the deterministic core is
  auditable. Right now that core and the LLM live in one crate, so "the scoring API
  has no network dependencies" is true of the *scoring crate* and false of the
  *binary that serves scores*. A crate boundary makes the claim structural.
- `api` becomes 824 lines, which is a file a new contributor can read in one sitting.
- Agent churn stops sharing a test module with the REST API's fixtures.

**Not worth doing: splitting for build times.** This is the argument I expected to
make, so I measured it before writing it:

```
rebuild after touching main.rs   2.49 s
rebuild after touching scoring   1.64 s
api test suite (59 tests)        0.08 s execution
```

Two and a half seconds. The test suite executes in 80 milliseconds. At this scale
the compile-time case for a crate split is **zero**, and asserting it would have
been repeating received wisdom instead of checking. If the file were ten times
larger the calculus changes; today it does not.

**Also not worth doing: splitting the REST handlers into modules.** Seven handlers
and their helpers in 824 lines, none over 94 lines, all sharing `card_for` and
`AppState`. Breaking that into `handlers/building.rs`, `handlers/search.rs` and so
on adds import ceremony and buys nothing a reader wants. The file is only awkward
because of what is stacked *on top* of it.

---

## The hardest question a reader can ask of this chapter

> *"Three thousand lines in one file is a smell. Don't explain it away — defend it
> or fix it."*

Half of it explains away legitimately and half of it does not, and it is worth
separating them because they get conflated in every review of this kind.

**What genuinely explains away:** 947 of those lines are tests, and in Rust unit
tests belong in the file. That is not 3,028 lines of anything. Of the remaining
2,082, the handler layer is 824 — an unremarkable size for nine routes, with no
function over 94 lines.

**What does not:** the other 1,258 lines are a different program. Async where the
rest is synchronous. Networked where the rest is local. Non-deterministic where
Chapter 2's entire thesis is determinism. Spending money where the rest reads a
read-only SQLite file. They share a file because of when they were written, and
nothing in the design says they should.

So the smell is real and it is misdiagnosed. The problem is not length; you can
read 2,082 lines. The problem is that a reader cannot tell from the crate boundary
which parts of this service can call the internet, and that is exactly the property
the project claims to care about most.

The defensible position, stated plainly:

1. **Extract `crates/agent`.** Not for tidiness or compile time — measured at 2.5
   seconds, that argument is dead. For the invariant: after the split, "nothing in
   the scoring path touches the network" is enforced by Cargo rather than asserted
   in a book.
2. **Add the schema-to-dispatch test.** Five lines. Closes the last unchecked
   hand-maintained correspondence in the file.
3. **Leave the REST layer alone.** It is 824 lines that do one thing, and every
   proposed reorganisation of it is motion.

And the thing this chapter should be read as conceding: I came to this file
expecting to find that its size hid defects, the way Chapter 4's `println!` and
Chapter 5's doc comment did. It mostly does not. The lock discipline is
compiler-enforced, the spend guard is symmetric and correctly ordered, the rejected
dependency is documented with its reason, and the tool dispatch is covered
tool-by-tool. The operational care in this file is higher than in the scoring crate
that Chapter 3 and Chapter 4 took apart — which is the opposite of what file size
would predict, and worth saying out loud.

---

*Next: **Chapter 7 — The Database Is the Deployment.** Why a 1.2 MB SQLite file
baked into the image is the architectural decision the rest of the system is
downstream of, and what it costs when the data goes stale.*
