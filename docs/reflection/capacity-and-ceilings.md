# What Could These Actually Hold?

*Game theory, for my own sake. If each of the four L2 projects got a foothold in its market —
who would the users be, how many, and what would the code do under them? Grounded in the
measured shape of each codebase, not in ambition.*

**The one-line finding:** in three of four projects there is no capacity ceiling because there
is no server. In the fourth there is a server, and **capacity is not its limit** — coverage
and cost are, by two orders of magnitude.

---

## 1. Resona — L2 Cycle 1

**Shape:** Tauri desktop app. whisper.cpp locally. **Zero network calls in the shipping app** —
grep for `fetch`/`http`/`reqwest` across the whole tree returns nothing, and no HTTP client is
even in `Cargo.toml`.

**Who it would serve:** the PRD names professionals, students, creators, non-native English
speakers, with privacy-sensitive users as the wedge. That wedge is the honest one: this is the
transcription tool for people who cannot send audio to a server — lawyers, clinicians,
journalists with sources, anyone under NDA. Otter and Deepgram cannot serve them at all.

**Capacity:** unbounded and meaningless. Every install is its own universe. A million users is
a million laptops doing their own inference; my infrastructure cost is $0 at any scale.

**The real ceiling is distribution, and it is at zero.** There is no CI, no LICENSE, no release
tags, and **no release build was ever produced** — the only binary on disk is a 14.2 MB debug
executable, despite `bundle.targets: "all"` being configured. The app cannot be installed by
anyone who is not me.

**Realistic user base if it shipped:** privacy-constrained professional transcription is a real
niche. Comparable indie tools (MacWhisper, Superwhisper) sit in the low tens of thousands of
paying users. **1,000–20,000 installs** is the plausible band, and it would be revenue-viable
at that size *if* payment worked.

**What actually blocks revenue, not scale:** `validate_license` is `key.starts_with("PRO-")`.
Anyone types `PRO-anything` and has Pro forever. The file itself says DEMO ONLY, and ADR-005
correctly states that revenue needs a server-signed entitlement. The diagnosis was right and
the work was deferred. **This is the whole business, and it is seven lines of unwritten code.**

---

## 2. SiteAssure — L2 Cycle 2

**Shape:** same single-device offline posture, deliberately. Plus a SHA-256 append-only hash
chain: `entry_hash = sha256(prev_hash + payload_hash)`, re-walked by `verify()`.

**Who it would serve:** construction and industrial site supervisors doing OSHA-required
documentation. But the hash chain reveals the *real* customer, and it is not the supervisor —
it is **the person who has to believe the supervisor.** An OSHA inspector. An insurer pricing
a policy. A lawyer in a dispute. `DATA_POLICY.md` already designs a redacted export packet for
exactly that reader.

**Capacity:** per-device, so again unbounded. But this is the one project where single-user is
not a limitation — **it is the product.** A tamper-evident log is more credible precisely
because it never left the device and no server could have rewritten it. Multi-tenancy would
weaken the claim.

**Market shape:** ~8 million US construction workers, but the buyer is the firm. Roughly
750,000 US construction establishments; the addressable slice is firms large enough to fear an
OSHA citation and small enough to lack enterprise EHS software — call it **50,000–150,000
firms**, at 1–20 seats each.

**Realistic if it got a foothold:** compliance software sells slowly and renews forever.
**500–5,000 firms** would be a genuine business at $30–100/seat/month.

**What blocks it:** four of eight Tauri commands return `Err("not implemented")` and all six
React screens return `null`. 551 lines of first-party Rust against 5,184 vendored. This is a
strong idea at prototype stage, and the vendored `wisper-core` still exports `download_url`
and `check_for_update` inside a project whose first non-negotiable is zero egress — that has
to go before anyone regulated installs it.

---

## 3. Ziqpu — L2 Cycle 3

**Shape:** the most *infrastructurally* mature. Seven crates, Postgres 16, twelve release
tags, 3-OS CI, CLA, DCO, an MCP server, a 69,458-city gazetteer, 5,271 tickers.

**Who it would serve:** astrology is a large consumer market — Co-Star claims tens of millions
of installs. The MCP server points somewhere more interesting though: Ziqpu is usable as a
*tool other agents call*, which makes developers a second audience.

**Capacity — and this one has a real, countable wall:**

| Constraint | Value | What it means |
|---|---|---|
| Postgres pool | `max_connections(5)` | 5 concurrent DB operations, total |
| LLM key | one `ANTHROPIC_API_KEY` per process | every user shares one rate limit and one bill |
| Auth | none | no way to attribute usage to anyone |
| Bind address | `0.0.0.0:8787` | reachable off-box, with nothing in front of it |

**Concurrency ceiling: roughly 5 simultaneous readings**, and the shared API key means user
#500 is throttled by user #499. **This is the only one of the four that would actually fall
over**, and it would do so at about a dozen concurrent users.

**The tell I keep coming back to:** `synastry_readings` is keyed
`PRIMARY KEY (user_chart_hash, choice_ticker)` — a genuine multi-user structure — and **no
code ever writes to it.** The multi-user design exists in the schema and nowhere else.

**Realistic foothold:** consumer astrology is winner-take-most and Co-Star owns it. The
defensible wedge is the **finance-astrology crossover** — 5,271 tickers is unusual and nobody
else has it — which is a niche of maybe **5,000–50,000 curious users**, or a developer-tool
play through MCP with a few hundred integrators. Either needs auth and a per-user key first.

---

## 4. HouseCheck — L2 Cycle 4

**Shape:** Rust/axum on Fly, 256 MB shared-cpu-1x, read-only SQLite baked into the image,
React on Vercel. Rate limit 10 req/60s per client. `ConcurrencyLimitLayer(64)`.

### Measured capacity — and this is the surprise

The read path was measured at **2.2 ms per Health Card** and 21 ms to score all 250. At 50%
headroom for a single shared vCPU that is ~227 cards/second.

| Session length | Peak-hour share of DAU | Sustainable DAU |
|---|---|---|
| 6 card views | 10% | ~1,360,000 |
| 6 card views | 20% | ~680,000 |
| 10 card views | 10% | ~820,000 |
| 10 card views | 20% | ~410,000 |

**Four hundred thousand to 1.3 million daily users on a $2/month virtual machine.** That is not
a typo, and it is the direct consequence of Chapter 1's architecture: everything expensive
happens once, at ingest, on a laptop. The serving path does four integer multiplications and a
SQLite lookup against a database small enough to sit entirely in page cache.

**So capacity is not the ceiling. It is not even close to being the ceiling.**

### Where the ceilings actually are

**1. Coverage — the binding constraint.**

| | Buildings | Artifact | |
|---|---:|---:|---|
| Today | 250 | 1.3 MB | one community district |
| One borough | 5,000 | 25 MB | still fits |
| **The cliff** | **40,000** | **203 MB** | **exceeds the 256 MB VM** |
| All HPD multifamily | 180,000 | 914 MB | needs a different design |

You cannot serve 400,000 people a product covering one neighbourhood. **The architecture that
makes the capacity absurd is the same one that caps the coverage** — a database baked into the
image is free to serve and impossible to grow past ~40,000 rows of this shape.

**2. LLM cost — the economic ceiling.** The read path is free; `/agent/chat` is not.
Parametric, at a plausible $0.004–0.008 per turn:

| DAU | Turns/user/day | Monthly LLM cost |
|---:|---:|---:|
| 1,000 | 0.3 | $36 – $72 |
| 10,000 | 0.3 | $360 – $720 |
| 100,000 | 0.2 | $2,400 – $4,800 |

*(Rates need verifying before anyone quotes these. The shape is the point: serving cards is
free, and the agent is the entire cost structure.)*

**3. The rate limit is not a scaling ceiling.** 10 req/60s per client key caps one abuser. It
does nothing to aggregate load, and `client_key` says so itself: *"a spend guard, not an
authentication boundary."*

### Realistic user base if it got a foothold

Bounded by the market rather than the machine. From the Research Notes: **33,210 units
available at a point in time**, 188,000–283,000 households entering the market annually. If
HouseCheck reached 5% of NYC movers in a year that is **~12,000 users/year** with sharp
seasonal peaks, using it for a few weeks each. Openigloo claims 3M+ reached over five years,
so **50,000–200,000 registered over several years** is the optimistic ceiling for NYC alone.

**The box could serve twenty times that on current hardware.**

---

## What the four have in common

Three of them cannot fall over because nothing is shared. The fourth can serve a city and is
limited by how much of the city it has data for.

**The pattern across all four is the same one from the arc reflection**: I build systems whose
*compute* scales effortlessly and whose *user model* does not exist. Resona and SiteAssure push
all cost to the client. Ziqpu shares one key and one 5-connection pool among everybody, which
is the one genuinely fragile design. HouseCheck precomputes everything so the request path is
nearly free.

That instinct is good engineering and it is why the capacity numbers are absurd in my favour.
It is also exactly why none of them can tell one user from another — **there is no per-user
state because I never built anything per-user.**

### If I only fixed one thing per project

| Project | The single unlock | Cost |
|---|---|---|
| **Resona** | A signed server entitlement, and actually cut a release build | days |
| **SiteAssure** | Finish the four unimplemented commands and the six null screens | weeks |
| **Ziqpu** | Per-user API keys and auth in front of `0.0.0.0:8787` | days |
| **HouseCheck** | Coverage past one district — the only thing between it and a real audience | one ingest run for a borough; a storage rethink past 40,000 |

None of those is a capacity problem. Every one is the same missing piece the arc reflection
found: **the user as a first-class thing the code knows about.**
