# Solution Design Sprint — HouseCheck

**Three sketches, one commitment.** Not the first solution that came to mind — three, each pushed
further from the obvious, then pressure-tested and scoped down to one MVP.

**Dated:** 2026-08-09. Living document, same standard as the Research, Framing and Problem
Definition notes — every figure carries its source, and where a number is derived rather than
sourced it says so.

**The problem this is solving**, committed in `problem-definition-notes.md`:

> A tenant lawyer preparing an HP action can see that a building has seven open hazardous
> violations but not what they are, because every tool in this space reports violation counts
> rather than the description text HPD already publishes — so they hand-copy conditions out of
> HPD Online for every client.

---

## Sketch 1 — The simplest thing that could work

**One interaction, and it already ships.** Type an address, open the Health Card. The only
change: the Condition pillar expands into the violations behind the number.

| | |
|---|---|
| **User does** | Types an address, taps the Condition score |
| **Product does** | Lists every open violation: class, published notice text, date |
| **Result** | "7 open Class C" becomes seven readable lines they can select and copy |

### Why it is easy

Measured before claiming it — 800 rows sampled live from HPD `wvxf-dwi5`:

| | |
|---|---|
| Rows carrying `novdescription` | **800 / 800 — 100% populated** |
| Mean / median length | 120 / 115 chars |
| p90 / max | 161 / 258 chars |
| Distinct texts | 83% unique |

The build is one column added to the SoQL select, one field on `Violation` (currently
`{ class, open, year }`), one render block in a section that already exists. **Scoring is
untouched** — descriptions are display-only, so the number cannot move, and the work the audit
did to make it correct is not at risk. No migration, because there are no accounts. Reversible.

### What it misses

**1. It costs two-thirds of the remaining coverage headroom.** 26,306 violations × ~120 chars
≈ **3.2 MB of text** against a 1.3 MB artifact.

| | Before | After (derived) |
|---|---:|---:|
| Per building | ~5.2 KB | ~17.6 KB |
| Buildings before the 256 MB VM breaks | ~40,000 | **~14,500** |

The cheapest fix to the committed problem and the coverage requirement are in direct
competition. Nothing in the earlier analysis surfaced this.

**2. It is not plain English.** Actual sample:

> `§ 12 M/D LAW DISCONTINUE THE STORAGE OF COMBUSTIBLE MATERIAL 100 CUBIC YARDS AT GASMETER
> ROOM AT CELLAR, SECTION AT WEST`

All caps, statute-prefixed, location-suffixed. **Ideal for the attorney** — it is what goes in
the petition. **Barely better than a count for the renter.** So the simplest solution serves the
committed user and does nothing for the mission user.

**3. It does not remove the copy-out.** They stop *hunting* for conditions; with no export they
still retype.

**4. Coverage still disqualifies it.** 250 buildings in one district is not a caseload.

**5. 83% unique means near-duplicates.** Ungrouped, a building with 33 violations renders as a
wall of nearly identical lines differing only by room.

---

## Sketch 2 — The full-featured version

The simple version makes a building *readable*. This makes it **citable** — something a lawyer
puts in front of a judge and opposing counsel can check.

| # | Adds | Wound |
|---|---|---|
| 1 | **Dual-register descriptions** — raw notice text preserved, plain-English condition beside it | plain English |
| 2 | **Verifiable evidence export** — condition history as a document with a hash chain proving it was not edited after retrieval | copy-out |
| 3 | **Per-user state** — saved buildings, caseload view, retrieval history | copy-out |
| 4 | **Owner/portfolio dimension** — join HPD registration so twelve buildings resolve to one landlord | Problem 3 |
| 5 | **MCP server** — HouseCheck as a tool another agent calls | reach |
| 6 | **Citywide coverage** — all ~180,000 HPD multifamily buildings | coverage |

**#2 is the one worth defending.** A count they hand-copy is unverifiable hearsay. A signed
packet is an exhibit.

### What it draws from earlier cycles

**Cycle 2 · SiteAssure is the architecture of #2, not an analogy.** It already implements
`entry_hash = sha256(prev_hash + payload_hash)`, append-only, re-walked by `verify()`, and its
`DATA_POLICY.md` already designs a redacted export packet for a third-party reader. That reader
was an OSHA inspector who has to be able to disbelieve the supervisor and check. **A housing
court judge is the same reader** — the audience designed for in Cycle 2 and the audience
committed to in Cycle 4 are the same shape.

**Cycle 1 · Resona supplies the signing.** Its license check was rebuilt this cycle on Ed25519
(`ed25519-dalek`, `verify_license_with(public_key_hex, key)`), verifying offline against a
compiled-in public key. Directly reusable for signing an export.

**Cycle 3 · Ziqpu supplies scale-out and MCP.** Seven crates, Postgres 16, a 69,458-city
gazetteer, 12 release tags, 3-OS CI — the only one of the four that has moved a large dataset
into a real database. Citywide coverage is a Ziqpu-shaped problem, and its MCP server makes #5 a
pattern already implemented rather than invented.

### Why it is hard in two weeks

1. **Coverage and descriptions fight each other.** Doing #1 and #6 together forces #6 to abandon
   the baked-artifact design — Postgres, a DB server, pooling, backups, secrets. That deletes the
   property the deck sells: *no DB server to breach, no write path to abuse.* An architectural
   inversion, not an increment.
2. **Per-user state is the wall hit four times.** A grep for `login`, `jwt`, `session`, `user_id`
   returns no implementation in any of the four cycles. And on a lawyer's tool, a saved caseload
   is client-adjacent data.
3. **The hash chain is only as good as what it attests.** SiteAssure's chain proves the log was
   not edited. It does **not** prove the data matched HPD at retrieval. That needs the ingest to
   record dataset version and retrieval timestamp per row.
4. **Plain-English mapping is where this could do harm.** Either an LLM pass over 26,306
   violations or a hand-built code→condition table validated by a housing lawyer. A wrong
   rendering on a legal-rights tool is exactly what the Guardrails work exists to prevent.

**Six subsystems, two weeks, one developer buys two.**

---

## Sketch 3 — The unconventional one

From Research Notes §6: *willingness to pay runs opposite to alignment.* Rentlogic's answer was
to sell certification badges to landlords, and it inherits a fatal flaw — **a landlord buys the
badge to advertise a good grade, so the buildings a renter most needs warning about are never
customers.**

> **Do not sell the landlord a badge. Sell them the right of reply — and publish their silence.**

HouseCheck stops being a scoring product and becomes **a public ledger of promises against
outcomes.**

> **Class C · no heat · opened 14 Mar**
> *Landlord response, 2 Apr:* "Boiler replaced, contractor invoice on file."
> *HPD record, 19 Jun:* still open. **78 days.**

The landlord pays to attach a response. They cannot delete or edit the city's record, and cannot
edit their own response once posted — appended, timestamped, hash-chained. Later HPD data
confirms or contradicts it automatically. **And a landlord who says nothing appears as "no
response" against every open violation: silence becomes a published data point.**

| | Sketch 2 | Sketch 3 |
|---|---|---|
| Who pays | nobody (unresolved) | the party being measured |
| What is scored | a building's condition | **an operator's credibility over time** |
| Non-participation | invisible | **the loudest signal** |
| Adverse selection | kills the payer model | **feeds it** |

### Kill conditions, named

- **Defamation exposure is real and not something this project is qualified to assess.** Needs a
  lawyer before a line of code. A hard gate, not a caveat.
- **Could become a landlord marketing channel** if responses are unmoderated.
- **Needs identity verification** — proving someone is the registered owner. None of the four
  cycles has that.
- **Needs citywide coverage** to mean anything.
- **Ethical hazard:** charging the measured party is structurally how credit bureaus are accused
  of operating. The defence is *money buys speech, never the score* — and if that line cannot be
  held under pressure, the idea should die rather than be compromised.
- **Most likely to be wrong:** that landlords would pay at all. Zero evidence. One conversation
  with a small property manager tests it.

---

## The decision

**Sketch 2 as the spine — descriptions plus a verifiable export — with one element lifted out of
Sketch 3 and the rest of Sketch 3 left on the shelf.**

The element taken from the ledger is its *insight*, not its mechanism: **an operator's track
record is what actually matters, and part of it is computable from HPD alone.** Every violation
carries an open date and, when resolved, a close date. So **days open** and **median
time-to-close per landlord** are derived fields — no landlord participation, no identity
verification, nobody's speech published.

That is the ledger's most valuable signal with none of its kill conditions. The defamation gate,
the would-landlords-pay assumption and the identity subsystem all evaporate, because the city's
own dates do the talking.

**Not taken: the payer inversion.** The most interesting idea on this project, and it needs a
lawyer before it needs a developer. Parked, not abandoned.

### MVP scope — the single core feature

> **A tenant lawyer opens a building, sees every open violation in the notice's own words with
> how long each has been open, and exports it as a file a stranger can independently verify was
> not altered after retrieval.**

The **export** is named as the core rather than the descriptions, deliberately. Descriptions
alone make a better lookup — incremental, and hard to distinguish from what exists. The export
changes what the tool *is*. And it cannot exist without the descriptions, so Sketch 1 sits inside
this scope rather than beside it.

**The demo moment:** export a building's record, hand the file to someone in the room, have them
change one character, watch verification fail.

#### In scope

| Area | Work |
|---|---|
| Ingest | fetch `novdescription`, open/close dates; **record dataset version and retrieval timestamp per row** |
| Model | `Violation` gains `description`, `opened_on`, `closed_on` |
| Card | open violations listed: class, raw notice text, **days open** |
| Derived | median days-to-close for this building's landlord, from HPD data only |
| Export | one document plus a hash chain over the rows, signed Ed25519 |
| Verify | a public checker that answers valid / tampered |

The timestamp row is not bookkeeping. Without dataset version and retrieval time, the signature
attests to a file rather than to a fact — security theatre.

#### Explicitly out

- **Plain-English normalisation.** The wound stays open on purpose; validating it needs a housing
  lawyer, not a sprint.
- Accounts and saved caseload — the four-cycle gap, too big to bolt on here.
- Citywide coverage — descriptions already move the ceiling to ~14,500 buildings. Fine at 250,
  but it must be **stated**, not discovered.
- Owner/portfolio join, and everything landlord-facing.

### What would prove this wrong, cheaply

1. **One call to a Legal Aid attorney.** If the count is sufficient, or an exported file does not
   fit how a case is actually built, the core feature is aimed at nothing. This should happen
   *before* the ingest change.
2. **One real ingest with descriptions on the 250**, and read the actual file size, rather than
   trusting the arithmetic above.

---

## What this sprint added that was not known before

1. `novdescription` is **100% populated**, mean 120 chars — the feature is viable, measured
   rather than assumed.
2. The text is **the notice's own language**, which serves the attorney and not the renter. That
   splits what looked like one feature into two.
3. Descriptions cost **~3.4× the artifact**, moving the coverage ceiling from ~40,000 buildings
   to ~14,500. The cheapest fix to the committed problem spends most of the remaining headroom.
4. **Days-open is free.** It falls out of data already ingested and gives an operator-level
   signal with no legal exposure. This is the single best value-per-effort item found.
5. SiteAssure's verifier audience and HouseCheck's committed user **are the same shape**, which
   makes an entire prior cycle reusable rather than merely referenced.
