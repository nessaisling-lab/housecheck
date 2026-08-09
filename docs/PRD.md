# HouseCheck

**Product Requirements Document — Net New Build**

**Build name:** HouseCheck
**Owner:** Aisling Leiva-Davila (backend & data) · Antonin (UI & UX) · Jagger (agent & research)
**Date:** 9 August 2026
**Version:** 2.0 — supersedes the capstone-period v1. Rewritten after the industry research,
market framing, problem definition and solution design sprint, all in `docs/classwork/`.

**In one sentence:** HouseCheck turns New York City's public housing records into one honest
0–100 Building Health Card, so the people whose safety, money or legal case depends on that
record can actually use it at the moment they need it.

---

## 1. Problem

A building's condition is a matter of public record, and the people whose safety, money or legal
case depends on that record cannot use it at the moment they need it.

Two groups hit this differently. A **renter** commits roughly $40,000 over a year after fifteen
minutes in a hallway, to a building with a decades-long public record spread across five
agencies. A **tenant lawyer** preparing a case can find out that a building has seven open
hazardous violations but *not what they are* — because every tool in this space reports violation
counts rather than the description text HPD already publishes. So they open HPD Online, key in
the borough-block-lot, and hand-copy conditions into the case file, once per client.

The root cause is the same in both cases: the data is **published but not legible**, and nothing
turns a count into a condition.

### Supporting context

- **11.1 million** HPD housing-maintenance-code violation records are public on NYC Open Data —
  and effectively unreadable at the moment of decision.
- **1.41% rental vacancy**; only **33,210** units are available at a point in time (NYC HPD,
  2023). Roughly **one renter in seventy-five** can act on this information at any moment, which
  is why reach has never been the constraint in this market — timing is.
- **Every existing product reports counts, not conditions.** We sampled 800 rows live from HPD
  `wvxf-dwi5`: the `novdescription` field is **100% populated**, mean 120 characters. The meaning
  is published and nobody surfaces it.
- **The count can be read backwards.** 603 Putnam Avenue scores condition 0 with 11 Class A, 22
  Class B and zero Class C — so a card can truthfully say "no hazardous violations" beside a
  floor-level score. Both numbers are correct; the sentence that reconciles them cannot be
  rendered without the descriptions.

---

## 1a. Opportunity

Make the city's own housing record usable at the two moments it matters — the signature and the
filing — for the roughly **188,000–283,000 households** who enter the NYC rental market each year
and the several hundred housing advocates who work those records daily. Solving it produces
something no competitor has: not a score, but a **citable** record.

### Market opportunity

- **91,918** NYC multifamily buildings (REBNY, *Data Over Rhetoric*, Feb 2026); **2,357,000**
  total housing units (2023 NYCHVS).
- Bottom-up, even optimistic adoption lands in **low single-digit millions of ARR in NYC** —
  a real business, not a venture-scale one. Rentlogic's twelve years at nine people is the
  evidence.
- **The global "proptech" figure is deliberately not used.** Five research firms put 2025 between
  $40.2B and $47.1B — a ~17% spread on the same year, and the category bundles investor analytics
  and building-management software that have nothing to do with this product.
- **Structural finding:** willingness to pay runs opposite to alignment. The closer a buyer's
  interest is to the tenant's, the less they will pay. Two companies have spent twelve years
  discovering this. It shapes every decision below.

---

## 1b. Users & needs

**Primary users: housing advocates.** Tenant lawyers, Legal Aid staff, tenant organisers and
housing court staff. Several hundred citywide at a countable number of named organisations. They
open building records **several times a day**, and they care about defensibility — every claim
they repeat, they have to stand behind.

**Secondary users: renters in market.** A household in the four-to-eight week window between
deciding to move and signing. ~33,210 in market at any moment. They need this **twice a decade**
and cannot articulate what they need, because they do not know the record exists.

**Why the primary is the less numerous group:** frequency drives quality. A user who opens the
tool twice a day finds every gap in a month, and their requirements are a **superset** of the
renter's. Building the harder one first makes the renter version a subtraction rather than a
rewrite. This is a design-target decision, not a revenue one — advocates are grant-funded and
sit at the very top of the alignment gradient.

### Key user needs

- As a **housing attorney**, I need to see what a building's violations actually *are*, not how
  many there are, because a petition has to plead conditions and dates rather than a total.
- As a **housing attorney**, I need a record I can hand to opposing counsel and have them verify
  independently, because an unverifiable printout is hearsay.
- As a **tenant organiser**, I need to know how long a landlord takes to close violations,
  because a pattern of neglect is the argument and a single building is an anecdote.
- As a **renter**, I need to know what I am signing into before I sign, because I get fifteen
  minutes and the consequence lasts a year.
- As a **rent-stabilised tenant**, I need to know my unit's status, because rights I do not know
  about are rights I cannot exercise.

---

## 2. Proposed solution

HouseCheck is a free web app that turns New York City's public housing records into one Building
Health Card per address. Users type an address and the system scores the building 0–100 across
four pillars — condition, legal, neighbourhood and accessibility — each traced back to the
government dataset it came from. Every open violation is listed in the wording of the city's own
notice, with how long it has been open, and the whole record can be exported as a file that
anyone can verify was not altered after retrieval. As a result, a renter can read a building's
history in the time they have, and an advocate can put that history in front of a court without
retyping it.

## 2a. Value proposition

Housing advocates and renters who struggle to read a building's record because the facts are
public but scattered across five agencies and written in violation codes use **HouseCheck**, a
free web tool, to see a building's real condition in seconds and export it as verifiable
evidence. Unlike existing services, which report *how many* violations a building has, HouseCheck
shows *what they are* in the city's own words, how long each has been open, and proves the
export was not tampered with — turning a number you have to trust into a record you can check.

## 2b. Top 3 MVP value props

- **The Vitamin (must-have baseline):** one address in, one honest 0–100 Health Card out, with
  every number opening to the public record behind it.
- **The Painkiller (solves the core pain):** every open violation shown in the notice's own
  words, with how long it has been open — so a count becomes a condition, and nobody has to
  hand-copy it out of HPD Online.
- **The Steroid (the magic moment):** export the record and hand it to someone who does not
  trust you; they change one character and verification fails. The record proves itself.

## 2c. Goals & non-goals

### Goals

- Make a building's real condition legible to a non-expert in under a minute, without an account.
- Give housing advocates a record they can cite — verifiable by a third party — so the work of
  assembling it stops being repeated per client.
- Surface landlord behaviour over time, not just a building's current state, using only dates the
  city already publishes.
- Never ship a number the product cannot source. An honest "unverified" beats a confident guess
  on a tool people make legal decisions with.
- Stay free at the point of use for renters and advocates.

### Non-goals

- **Plain-English rewriting of violation text.** The notice's own wording ships; translating it
  needs a housing lawyer to validate a code→condition mapping, and a wrong rendering on a
  legal-rights tool does more harm than no rendering. Deferred until it can be checked, not
  until there is time.
- **User accounts and saved caseloads.** On a lawyer's tool, saved work is client-adjacent data.
  Getting it wrong is worse than not having it, so it waits for a design that deserves it.
- **Legal advice of any kind.** The product explains what a rule says and cites the statute. It
  never says what to do, and never predicts an outcome. This is a permanent non-goal.
- **Selling anything to landlords in this version.** The one revenue model proven in this market
  — certification badges — has adverse selection built in: the worst buildings never buy.

## 2d. Success metrics

Deliberately chosen to be measurable **without per-user tracking**, because the product has no
accounts and no analytics by design. Every metric below is a server-side aggregate or a count of
an artifact produced.

| Goal | Signal | Metric | Target |
|---|---|---|---|
| Legibility | People open the detail, not just the score | Share of card views that expand the violation list | >40% of card views within 60 days of launch |
| Advocate adoption | Advocates produce citable records | Exports generated per week | >50/week by day 90 |
| The export is trusted | Third parties actually check it | Verifier runs against exported files | >1 verification per 10 exports |
| Coverage is sufficient | Lookups find the building | Share of address lookups that resolve to a covered building | >80% (today: one community district) |
| Honesty holds | No unsourced claim ships | Fields rendered as "unverified" rather than guessed | 100% of unsupported fields; zero fabricated values |
| Stays free and cheap to run | Serving cost stays near zero | Monthly infrastructure cost at 10,000 monthly users | <$50/month excluding LLM endpoints |

---

## 3. Requirements

### User Journey 1: A housing attorney building a case

**Context:** This is the primary user and the one who defines quality. They need conditions, not
counts, and they need to be able to defend every claim they repeat. Optimising for defensibility
and for work that compounds instead of repeating per client.

**Sub-journey: Finding the building's real condition**

- [P0] User can look up a building by address without creating an account.
- [P0] User can see every open violation listed individually, in the wording of the city's own
  notice.
- [P0] User can see how long each violation has been open, in days.
- [P0] User can see which dataset and which retrieval date each figure came from.
- [P1] User can see the median time this landlord takes to close a violation.
- [P1] User can filter the violation list by class (A, B, C).
- [P2] User can group near-identical violations that differ only by unit or room.

**Sub-journey: Producing a citable record**

- [P0] User can export a building's violation record as a single file.
- [P0] The export states the dataset version and the exact time the data was retrieved.
- [P0] The export carries a hash chain over its rows and is signed, so alteration is detectable.
- [P1] User can restrict the export to a date range.
- [P2] User can export multiple buildings in one file.

**Sub-journey: Verifying a record someone else produced**

- [P0] Any person can submit an exported file and be told whether it is intact or altered.
- [P0] Verification works without an account and without contacting HouseCheck's servers.
- [P1] Verification names which part of the file failed, not just that it failed.

### User Journey 2: A renter deciding on an apartment

**Context:** Fifteen minutes, on a phone, in a hallway, with other applicants in the room. The
constraint is time and comprehension, not depth. This journey mostly reuses Journey 1's data with
less of it shown.

**Sub-journey: Reading the building in under a minute**

- [P0] User can get a 0–100 Health Card from an address in seconds, with no account.
- [P0] User can see the four pillar scores separately, so one bad pillar is visible rather than
  averaged away.
- [P0] User can see a plain statement of what is wrong, including the count of open violations
  and how many are hazardous — so "no hazardous violations" can never be read as "this building
  is fine."
- [P0] User is told when a fact is unverified rather than shown a guess.
- [P1] User can read the whole card on a phone, including with a larger text size.
- [P2] User can compare two buildings side by side.

**Sub-journey: Understanding their rights**

- [P0] User can see whether the building is likely rent-stabilised, or that the record cannot
  support the claim.
- [P0] User can ask questions about the building and get answers drawn only from that building's
  record, with a citation for every legal claim.
- [P0] User is given a named free legal hotline with every legal answer.
- [P1] User can reach a verified referral directory of tenant organisations.

### User Journey 3: An operator keeping the data honest

**Context:** Not an end user, but the requirements exist because the product's credibility
depends on them. Every one of these was written after a real failure.

- [P0] Ingest fails loudly rather than silently truncating a paged dataset.
- [P0] The database records its own provenance — sources, retrieval time, snapshot year — and
  serves it at a public endpoint.
- [P0] The API refuses to start on an empty database rather than serving 404s under a green
  health check.
- [P1] Ingest re-runs on a schedule instead of shipping a point-in-time snapshot.
- [P1] Excluded record classes are stated on the card, not silently dropped.

---

## 4. Appendix

### Tech stack

| Layer | Choice | Why |
|---|---|---|
| API | Rust + Axum, five crates (`model`, `scoring`, `store`, `ingest`, `api`) | Scoring is a pure function of the record, so it is testable without a server or a database |
| Data | SQLite, baked into the container image, opened read-only | No database server to breach and no write path to abuse |
| Hosting (API) | Fly.io, 256 MB shared-CPU | Measured at 2.2 ms per card; capacity is not the constraint |
| Frontend | React 19 + Vite + TypeScript + Tailwind | Existing; deployed on Vercel |
| Agent | LLM with tool access restricted to the building's own record, plus search limited to nine government and academic domains | It answers from the record or says it does not know |
| Export signing | Ed25519 (`ed25519-dalek`), hash chain `sha256(prev_hash + payload_hash)` | Both patterns already built in earlier cycles — Resona's licence verification and SiteAssure's audit chain |
| Sources | HPD, 311, PLUTO, DOB, DOHMH, Census ACS5, DHCR, JustFix | All public, all named, nothing scraped or behind a login |

### Technical constraints

- **The coverage ceiling is smaller than it first appeared.** An earlier estimate put the ceiling
  at ~14,500 buildings once violation descriptions were added, implying citywide coverage
  required replacing the storage design. That assumed raw text. Measured: descriptions compress
  **9.9×** (statute-templated), only **25.6%** of the 11.2M citywide violations are open
  (2,858,719), and storage costs **48 bytes per violation row** in the current artifact.
  A citywide artifact lands near **240 MB compressed** against ~690 MB raw — so the read-only
  baked-artifact design **survives**, on a 512 MB machine at roughly $3–4/month, keeping the
  "no DB server to breach" property. Full working and the rejected alternatives in
  `docs/design/database-layer.md`. Escape hatch if it ever passes ~800 MB: SQLite over HTTP
  range reads from object storage, not Postgres.
- **No telemetry, by design.** Every success metric above is a server-side aggregate precisely
  because there is no per-user tracking to draw on.
- **The rate limit is a spend guard, not an authentication boundary** — it caps one abuser and
  does nothing to aggregate load.

### Open questions

1. **How long does the manual HPD Online pass actually take?** The most useful number not held.
   One conversation with a Legal Aid paralegal produces it, and it should happen before the
   ingest change.
2. **Do advocates already have a private workaround?** If so, the core requirement is much weaker
   than it looks from outside.
3. **Would a renter act on a bad score at the moment of decision, or sign anyway?** The
   load-bearing unknown. Openigloo reached 3M+ renters and still pivoted to brokerage.
4. **How should the storage layer change to reach citywide coverage while staying free?** Named
   above as the largest architectural question.
5. **Reconcile the rent-burden figures** — 51.6% (RGB 2026) against 29.5% (2023 NYCHVS). Very
   likely different populations rather than a contradiction, but it must be established before
   either is cited.

### Supporting documents

All in this repository:

- `docs/classwork/industry-research-notes.md` — landscape, players, the alignment gradient
- `docs/classwork/market-framing-notes.md` — who else has this problem, and how many
- `docs/classwork/problem-definition-notes.md` — the committed problem statement
- `docs/classwork/solution-design-sprint.md` — three sketches, the decision, MVP scope
- `docs/BACKLOG.md` — the live task list
- `docs/reflection/capacity-and-ceilings.md` — measured capacity and where the real limits are
