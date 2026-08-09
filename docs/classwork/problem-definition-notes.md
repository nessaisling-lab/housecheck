# Problem Definition Notes — HouseCheck

**Going deep:** not a category of problems, a concrete situation where something goes wrong for
real people in a real way. The goal is one sentence specific enough to build from.

**Started:** 2026-08-09. Living document, same standard as the Research and Framing Notes —
every figure carries its source, and where a number is derived rather than sourced it says so.

---

## The standard this block has to meet, and where it currently falls short

The brief asks for **primary framing**: who specifically, in what situation, and why it matters
to them. Secondary research tells you what exists; primary framing tells you what is missing.

What I have is the public record, the competitive landscape, and first-hand knowledge of my own
build — which is secondary research plus engineering evidence. **I have not interviewed a tenant
lawyer, sat with an organiser, or watched a renter at a viewing.**

So each problem below separates two things:

- **Evidenced** — what I can point at: a published dataset, a field my ingest does or does not
  fetch, a sourced figure.
- **Needs primary contact** — what I would be inventing if I asserted it.

In particular I have not put a time figure on any of these. "Twenty minutes per patient shift"
is the right *shape* of claim, and I do not have the measurement that would earn it. Producing
a plausible number to match the shape is the exact failure this project exists to argue against.

---

## Problem 1 — The count is not the argument

**Who:** a housing attorney at Legal Aid, or a paralegal at Housing Court Answers, building an
HP action to force a landlord to make repairs.

**When:** at client intake, and again when drafting the petition. Several times a day, every
working day.

**The situation, concretely.** Every tool in this landscape — Rentlogic, Openigloo, and mine —
reports that a building has *seven open Class C violations*. That number cannot go in a filing.
A petition has to name conditions: no heat, no hot water, a rodent infestation, a specific
defect at a specific unit, unresolved since a specific date.

HPD **already publishes exactly that.** Every Notice of Violation carries description text. So
the attorney takes the count, opens HPD Online, keys in the borough-block-lot, pages through
the violation records one at a time, and hand-copies the descriptions into the case file. The
aggregate tool handed them a number and then sent them to fetch the meaning by hand.

**Cost of not solving it.** The labour is per-client and never compounds. Nothing the attorney
assembles for 689 Myrtle Avenue on Tuesday is available to the organiser working the same
building on Thursday. Multiply by hundreds of practitioners citywide, daily, against a corpus
of **11.1M published records** that already contains the answer.

**Evidenced.** HPD publishes the NOV description field. My own ingest pulls 26,306 violations
across 250 buildings and does not fetch it — `crates/model/src/lib.rs` defines
`Violation { class, open, year }` and nothing more, so the agent can say how many violations
exist but not what they are. No product found in the landscape closes this.

**Needs primary contact.** How long the manual pass actually takes; whether firms have already
built private workarounds that are invisible from outside.

### The same failure, in our own product — found 9 August 2026

Reviewing the pitch deck turned up an instance of this problem inside HouseCheck itself, which
is worth recording because it is evidence rather than embarrassment.

A Health Card on the slide titled *"Every number traces to a public source"* shows:

> **Condition — 1** · "No hazardous violations"

That reads as a contradiction: a floor-level score next to a clean-sounding caption. It is not.
Checked against the live API, **603 Putnam Avenue** is the same shape — `score.condition = 0`
with `open_violations = { a: 11, b: 22, c: 0 }`. "Hazardous" means **Class C**, of which there
are none, while thirty-three Class A and B violations drive the score to zero. Both numbers are
correct and sourced.

**What is missing is the sentence that reconciles them:** *33 open violations, none of them
Class C.* The card shows a count fact and a score fact and never the meaning that connects
them — so a reader either assumes a bug or, worse, reads "no hazardous violations" as "this
building is fine."

This is Problem 1 in miniature, in our own UI, on the slide that claims traceability. A count
without its meaning is not merely less useful than the description; it can be **read backwards.**
That raises the stakes on the committed problem: fetching HPD's description text is not only a
feature for attorneys, it is a correctness fix for the renter-facing card.

---

## Problem 2 — Fifteen minutes against a forty-year record

**Who:** a household viewing an apartment, standing in a hallway with a broker and — at 1.41%
vacancy — other applicants in the room.

**When:** at the viewing. The search runs four to eight weeks; the decision itself takes
minutes, and hesitating loses the unit.

**The situation, concretely.** They are about to commit roughly **$40,000 over the next year**
(derived: median asking rent × 12) to a building whose condition is a matter of decades of
public record across five agencies — HPD, DOB, DOF, DOHMH and 311. To check it in that hallway
they would need to know that a Class C violation means "immediately hazardous," know the
building's BBL, and know which of three portals holds which fact. None of that is available to
a layperson under time pressure, so the common behaviour is **sign without checking** — not
from carelessness, but because checking is not possible at the only moment it would matter.

**Cost of not solving it.** They move into unresolved hazardous conditions they had a legal
right to know about. Separately, roughly **1M rent-stabilised or rent-controlled units housing
~2.5M people** carry rights that can only be exercised by a tenant who knows the unit's status,
and stabilisation status is not something a broker volunteers.

**Evidenced.** 1.41% vacancy, 33,210 units available at a point in time (NYC HPD, 2023); 11.1M
violation records (NYC Open Data); the three-portal structure, which I hit personally while
building the ingest.

**Needs primary contact — and this is the load-bearing unknown of the whole project.** Whether
a renter would *act* on a bad score at that moment, or sign anyway because there is no
alternative unit. Openigloo reached 3M+ renters and still pivoted to brokerage, which is at
least consistent with the answer being "they sign anyway."

---

## Problem 3 — A landlord is invisible; only buildings are visible

**Who:** a tenant organiser at Met Council or a tenant union choosing where to run a campaign;
and an investigative housing reporter.

**When:** at the start of a campaign or a story, then continuously as the picture goes stale.

**The situation, concretely.** The record is published **per building**. A landlord operating
twelve buildings appears as twelve unrelated records, because the linking information —
registered owner and officer names — lives in a different HPD dataset than the violations. So
the organiser reconstructs the portfolio by hand in a spreadsheet, and it is stale the week
after they build it. The pattern that would justify a campaign or carry a story — *this
operator's entire portfolio degrades the same way* — has to be rebuilt from scratch by every
person who wants it.

**Cost of not solving it.** Organising effort gets aimed at buildings rather than at the
operator making the decisions, and the systemic pattern stays invisible.

**Evidenced.** HPD registration data carries owner and officer contacts as a separate dataset.
My own build keys on BBL and has no owner dimension at all.

**Confidence: lower than the other two.** The organiser's workflow here is inferred from the
landscape rather than observed. Flagged rather than dressed up.

---

## Which one to commit to

Not the biggest — Problem 2 affects far more people. The one where every precondition is
already satisfied:

| | P1 — meaning | P2 — the moment | P3 — portfolios |
|---|---|---|---|
| Data exists and is published | yes | yes | yes, separate dataset |
| Users are reachable | yes — hundreds, at named orgs | **no** — 1 in 75, unaddressable | few, but reachable |
| Known they would use it | needs confirming | **the open question** | inferred |
| Closable this cycle | yes — one ingest field | no — a distribution problem | new dataset plus a join |

Problem 2 is the mission, and it is a **distribution** problem rather than a data one: the
information is already legible on the card. Nothing in the ingest fixes it.

Problem 1 is a **data** problem, closable by fetching a field the ingest currently skips, for
users reachable with a phone list. It is also the problem the daily user has and the occasional
user cannot articulate — a renter does not know to ask for violation descriptions, because they
do not know the field exists.

### The problem statement

**Final** — tightened into the template shape after review. The earlier draft implied the
underlying reason instead of stating it; the template asks for the *because*.

> **A tenant lawyer preparing an HP action can see that a building has seven open hazardous
> violations but not what they are, because every tool in this space reports violation counts
> rather than the description text HPD already publishes — so they hand-copy conditions out of
> HPD Online for every client.**

Reading it against the template:

| Element | In the sentence |
|---|---|
| **Who** | a tenant lawyer |
| **When** | preparing an HP action |
| **What goes wrong** | can see the count, not the conditions |
| **Because** | every tool reports counts, not the description text HPD already publishes |
| **Cost** | hand-copies conditions out of HPD Online, once per client |

**On the cost clause:** "for every client" is deliberately the whole of it. It carries the
shape of the cost — per-client, non-compounding, unbounded by caseload — without asserting a
duration I have not measured. The measurement is open question 1 below, and it stays there
until someone tells me the number.

**What makes this buildable:** one person, one moment, one artefact that already exists, one
specific missing step. It is falsifiable — if attorneys say the count is enough, the statement
is wrong and I will have learned that for the price of a phone call rather than a sprint.

**The earlier draft, kept for the record:**

> ~~A tenant lawyer preparing a case can find out that a building has seven open hazardous
> violations, but not what they are, so they hand-copy conditions out of HPD Online for every
> client — even though the city already publishes the text.~~

The difference is not cosmetic. "Even though the city already publishes the text" is an irony;
"because every tool reports counts rather than the text HPD publishes" is a **cause**, and a
cause is the thing a build decision can attach to.

**What is still unresolved:** it sits at the very top of the alignment gradient from the
Research Notes §6 — the group with the highest need and the least ability to pay. Choosing it
improves the product without clarifying the business, which is a deliberate trade and not an
oversight.

---

## What this changes in the build

| | Status before this block | After |
|---|---|---|
| HPD violation descriptions | listed as gap §4.5 | **committed — the problem statement rests on it** |
| `Violation` struct | `{ class, open, year }` | needs a description field, and the ingest needs to fetch it |
| Owner / portfolio dimension | not identified as a gap | **new gap** — recorded, not committed |
| Coverage past one district | blocking for professionals | unchanged, still blocking |

---

## Open questions this block leaves

1. **The time cost of the manual pass.** The single most useful number I do not have. One
   conversation with a Legal Aid paralegal would produce it.
2. **Whether attorneys already have a workaround.** If a private tool exists, Problem 1 is much
   weaker than it looks from outside.
3. **Whether a renter acts on a bad score.** Carried forward unresolved from the Framing Notes;
   it governs whether Problem 2 is ever worth building for directly.
4. ~~**Whether HPD's description text is usable as published**~~ — **partially answered
   9 August 2026.** Sampled 800 rows live from HPD `wvxf-dwi5`: the `novdescription` field is
   **100% populated** (800/800), mean 120 chars, median 115, p90 161, max 258, 83% distinct.

   It is usable **for the committed user and not for the mission user.** The text is the
   notice's own language — all caps, statute-prefixed, location-suffixed:

   > `§ 12 M/D LAW DISCONTINUE THE STORAGE OF COMBUSTIBLE MATERIAL 100 CUBIC YARDS AT
   > GASMETER ROOM AT CELLAR, SECTION AT WEST`

   For an attorney that is ideal, because it is what goes in the petition. For a renter it is
   barely better than the count. So shipping descriptions raw serves Problem 1 and leaves
   Problem 2 untouched.

   **And it collides with coverage.** 26,306 violations × ~120 chars ≈ 3.2 MB of text against a
   1.3 MB artifact — roughly 3.4×, moving the 256 MB ceiling from ~40,000 buildings to
   **~14,500** (derived; confirm with a real ingest before it drives a decision). The cheapest
   fix to the committed problem spends two-thirds of the remaining coverage headroom, and
   nothing in the earlier analysis surfaced that.

5. **Whether descriptions need grouping before display.** At 83% distinct, near-duplicates
   differ only by room or section, so a building with 33 open violations renders as a wall of
   nearly identical lines unless they are grouped by condition.

---

## Sources

Figures reused from the Research Notes and Market Framing Notes in this folder; see those
documents for full citation.

- 1.41% rental vacancy, 33,210 units available — NYC HPD, 2023
- 11.1M HPD violation records — NYC Open Data
- ~1M rent-stabilised/controlled units, ~2.5M tenants — NYCHVS-derived
- Openigloo pivot to brokerage — *The Real Deal*, 28 Aug 2025
- Our own build: 250 buildings, 26,306 violations, one community district
