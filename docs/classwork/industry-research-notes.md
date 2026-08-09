# Residential Rental Transparency — Research Notes

**Industry:** property data pointed at the tenant, not the owner.
**Researched:** 2026-07-29, extended 2026-08-08. **Living document** — the open questions in
§7 are the current edge, not a closing summary.

**Standard used throughout:** every figure carries its source, and confidence is stated where
a source is weak. Nothing here is estimated and then presented as measured. Where a claim
comes from our own build, it is marked as such and can be checked against a live URL.

---

## 0. What this industry is, precisely

Property data is a large, mature, well-funded industry — CoStar, Zillow, every brokerage CRM,
every insurer's risk desk. Almost all of it is sold to the people who **own, finance, or
transact** buildings.

This document is about the sliver pointed the other way: the same public records, sold or
given to the person about to **sign a lease** on the building.

That boundary is the whole thesis, and it is why the global "proptech" figure is not used
here — see §5.

**Evidence we can point at:** we built the thing. HouseCheck serves 250 Bedford-Stuyvesant
buildings, scores each on four axes from eight municipal sources, and is live at
`housecheck-wine.vercel.app` with an API at `housecheck-nessa.fly.dev`.

---

## 1. Major players

Two companies have built close variants of this in New York. Seven years apart, independently,
in the same city.

### Rentlogic — the closest analogue that exists

| | |
|---|---|
| Founded | 2013, New York City |
| Team | 9 employees (2025) |
| Stage | Seed |
| Product | Grades every NYC apartment building **A / B / C / F** on health and safety, scoring ~150 variables from city agency data plus physical inspections |
| Renters pay | Free |
| Revenue | Landlords buy certification, signage and marketing: **$99–$1,499 per building** by size — roughly **20¢ per unit per month** (~$2.40/unit/yr) |
| Funding | **$2.4M seed** (2018) — Urban-X, Urban.Us, Kairos, Edgar Bronfman Jr. |

**Read:** the model works and it is small. Twelve years, nine people, still seed-stage.
Published pricing is 2018-era; nothing current is public, so treat the per-unit figure as a
floor-level indication rather than today's rate card.

### Openigloo — reach, then a pivot

| | |
|---|---|
| Founded | 2020, Brooklyn. CEO Allia Mohamed |
| Reach | States it has supported **3M+ NYC renters** |
| Product | Tenant reviews plus city data — violations, bedbug complaints, eviction filings, litigation history; building scores for maintenance, heat and water pressure, pests, landlord responsiveness |
| Investors | Trend Forward Capital, Gutter Capital, MetroCap Partners |

In August 2025 *The Real Deal* published "Openigloo won over tenants. Can it do the same with
landlords?" The company now runs two revenue lines: **renter subscriptions** for depth of
access, and **brokerage** — exclusive listing agreements, application parsing, income
verification, credit checks, and rent guarantees, at a fee starting near half of one month's
rent.

A third-party aggregator estimates annual revenue near $1.37M. That is a scraped estimate with
no disclosed methodology — **low confidence, do not cite it externally.**

### Adjacent, and bigger: institutional property risk

- **Cape Analytics** — agreement to be acquired by **Moody's**, announced 13 January 2025.
  Price undisclosed; Moody's stated it was not expected to have a material impact on results,
  which bounds the size.
- **ZestyAI** — property-level risk to P&C insurers and regulators; added a former Verisk CEO
  to its board.
- **Verisk / BuildFax** — building-permit-derived property intelligence for insurers.

---

## 2. What those players tell us — the finding that matters most

Two independent teams, seven years apart, built a renter-facing building-data product in the
same city. **Neither made the data itself the business.** One sells badges to the landlords
being graded. The other moved to where a transaction happens and money changes hands.

> **The scoring engine is a customer-acquisition layer, not the product.**

This is the most useful sentence in this document and it argues against our own build. It is
recorded first rather than buried, because a project whose entire premise is that every number
should be checkable does not get to hide the number that is inconvenient.

**A structural problem worth naming**, visible in Rentlogic's model: a landlord buys the badge
to advertise a *good* grade. The buildings with the worst records — the ones a renter most
needs warning about — will never be customers. Revenue concentrates among the landlords who
least need scrutiny.

---

## 2b. The landscape, by what each player actually does

The two direct comparables are the closest analogues, but they sit inside a wider ecosystem.
Mapping it by **function** rather than by company name shows where the tenant-facing slice
actually is — and how thin it is.

| Layer | Who operates here | What they sell | Who pays |
|---|---|---|---|
| **The record itself** | NYC HPD, DOB, DOF, DOHMH, 311, US Census | Nothing — statutory publication | Taxpayers |
| **Bulk redistribution** | NYC Open Data / Socrata, CivicDashboards | Access and APIs | Free / platform contracts |
| **Landlord & investor analytics** | CoStar, Yardi, RealPage, PropertyShark | Portfolio, comp and underwriting data | Owners, brokers, lenders |
| **Institutional risk** | Cape Analytics (→ Moody's), ZestyAI, Verisk/BuildFax | Physical & catastrophe risk scores | Insurers, reinsurers |
| **Listings & transaction** | StreetEasy, Zillow, Apartments.com, brokerages | Attention, leads, lease transactions | Landlords, brokers, renters |
| **Tenant-facing building record** | **Rentlogic, Openigloo, HouseCheck** | A read on the building itself | *Unresolved — see §2* |
| **Advocacy & enforcement** | Legal Aid, Housing Court Answers, Met Council, HPD enforcement | Representation, referral, inspection | Grants, government |

**The observation that falls out of the table:** every layer has a settled payer except the
one this document is about. Listings monetise the transaction. Analytics monetise the owner.
Risk monetises the insurer. The tenant-facing row is the only one where the question "who
pays" is still open after twelve years and two companies.

---

## 3. The people this industry serves

Ranked by evidence rather than by how appealing they are to sell to.

| Segment | Reach | Willingness to pay | Evidence |
|---|---|---|---|
| **Renters** | ~2.5M NYC tenants; Openigloo claims 3M+ reached | Very low | Both comparables give the data away free. Openigloo charges for *depth*, not access |
| **Landlords / property managers** | 91,918 multifamily buildings | Low–moderate | Proven — Rentlogic's entire revenue line. Adverse selection applies (§2) |
| **Brokers / leasing** | — | Moderate | Openigloo's pivot. The FARE Act (eff. June 2025) moved broker fees to landlords, changing who buys tools that close a lease |
| **Legal services, tenant orgs, city agencies** | — | Low, but grant-funded | Aligned with the referral directory HouseCheck already ships. Credibility over revenue |
| **Insurers / lenders / investors** | — | Highest | Real exits exist (§4), but a different dataset and a mission conflict |

**The person at the centre of it.** A New York renter commits roughly **$40,000 over a year**
to a building they know almost nothing about. They get fifteen minutes and a signature. The
building has a forty-year public record.

Roughly **1M** rent-stabilised or rent-controlled apartments house about **2.5M** tenants who
hold rights they can only exercise if they know the building's status.

### The denominator the behavioural question forces

A tool used **at the moment of decision** does not serve a population. It serves a *moment*,
and the moment is far smaller than the population.

| Framing | Count | Source |
|---|---:|---|
| NYC renters (population framing) | ~2,500,000 | NYCHVS-derived |
| Total housing units | 2,357,000 | 2023 NYCHVS |
| **Units actually available at a point in time** | **33,210** | NYC HPD, 1.41% vacancy 2023 |

**The in-market moment is about 1.3% of the population framing — roughly 1 in 75.**

That is not a rounding note, it is the shape of the business. Two independent routes agree on
it: the sourced vacancy figure says 33,210 units are available at once, and an
8–12% annual turnover model on 2,357,000 units implies **21,800–32,600 households** searching
in any six-week window. Different methods, same order of magnitude.

**This is the most probable explanation of Openigloo's pivot.** They reached 3M+ people and
still left for brokerage. Reach was never the constraint — the constraint is that only about
one in seventy-five of those people is in a position to *act* on the information at any given
time, and the person who is in that position is, by definition, standing next to a
transaction. Openigloo did not abandon the audience. It followed the audience to the moment
where money changes hands.

**What this means for who we count as served:** the honest primary user is not "NYC renters."
It is *a household in the 4–8 week window between deciding to move and signing*, and secondarily
*a tenant already in a building who has a problem now* — a different moment, with different
urgency, which is the one the agent and the legal-referral directory actually serve.

### The workers this industry serves — an underserved segment we did not design for

The question "who does this industry serve" has a second answer we had not mapped, and it is
where the record is used *repeatedly* rather than once:

| Worker | What they do with building records | Frequency |
|---|---|---|
| Tenant lawyers, Legal Aid staff | Establish a building's violation history for a case | Daily, per client |
| Tenant organisers | Identify buildings and landlords worth organising around | Weekly |
| Housing Court staff and judges | Verify conditions claims | Per case |
| HPD inspectors | Prioritise where to inspect | Daily |
| Journalists and researchers | Find patterns across portfolios | Per story |
| Property managers and supers | Track their own open violations | Ongoing |

This group is small, mostly not able to pay, and **uses the data far more often than any
renter does.** A renter needs this twice in a decade. A tenant lawyer needs it twice a day.

Nothing found in this research is built for them. Rentlogic sells to landlords, Openigloo to
renters and now to landlords again. The repeat user with the highest need and the lowest
willingness to pay is the one nobody serves — which is a real finding, and it is not obviously
a business.

---

## 4. Gaps

### The gap the product exists for

The records are public and unusable at the moment of decision. NYC Open Data holds **11.1
million** HPD violation records. Reading them requires knowing that a Class C violation is
"immediately hazardous," that a BBL is a borough-block-lot key, and which of three portals
holds which fact. That is the gap between *published* and *legible*.

### The genuine white space

Searching specifically for a vendor selling **code-violation or landlord-behaviour risk scores
to insurers or lenders** returned no established player. Every institutional property-risk
company found models **physical and catastrophe** risk — roof condition, vegetation, wildfire,
hail, wind, water — largely from aerial and satellite imagery. None sells habitability.

Stated carefully: absence of evidence from public search is not evidence of absence. This is a
lead, not a conclusion.

### The gap that is a conflict, and has to be decided on purpose

Insurers are beginning to explore **habitability exclusions** — provisions allowing a claim to
be denied where a property is uninhabitable due to code violations or deferred maintenance.

That is the natural buyer for precisely this dataset. It is also the exact inversion of the
product's purpose. HouseCheck exists so a renter knows what they are signing into. Sold to that
buyer, the same records help an insurer refuse a claim on the same bad building, and the party
harmed is the tenant.

Recorded here rather than omitted because it is the highest willingness-to-pay path found.

**Cleaner variants of the same path**, where the buyer's incentive points the same way as ours:

- **Lenders** pricing multifamily credit risk — violation history is a real default signal, and
  an accurately-priced loan does not require harming a tenant.
- **City agencies** targeting enforcement — the enforcement body already holds the data; the
  value is prioritisation.

### What is missing or underserved — the short answer

Five things, ordered by how confident the evidence is rather than by how appealing they are.

1. **A settled payer for the tenant-facing layer.** Every other layer of the landscape table in
   §2b has one. This layer has had two serious attempts over twelve years and neither resolved
   it. This is the strongest finding in the document and it is a warning, not an opening.
2. **The repeat user nobody builds for.** Tenant lawyers, organisers, court staff and
   inspectors use building records daily; renters use them twice a decade. Every product found
   is built for the infrequent user. The frequent user has the highest need and close to zero
   budget.
3. **Habitability as a risk signal.** Institutional property-risk vendors model roofs, wildfire
   and hail from imagery. None found sells code-violation or landlord-behaviour risk. Genuine
   white space — with the conflict below attached to it.
4. **Reaching the renter in the 4–8 week window.** Not a data gap, a distribution gap. The
   information exists and is now legible; the unsolved part is being present at the moment one
   in seventy-five people is deciding.
5. **Violation *meaning*, not just violation counts.** Every product in this space, ours
   included, reports how many violations a building has. HPD publishes what each one *is*.
   Nobody found turns "7 open Class C" into "no heat, twice, unresolved since March."

   **Sharpened 9 August 2026** (see `problem-definition-notes.md`): the reason this gap is
   larger than it looks is that the count is not merely *less useful* than the descriptions —
   for the highest-frequency user it is **unusable**. An HP action has to plead conditions, not
   totals. So a housing attorney takes the aggregate number, opens HPD Online, keys in the BBL
   and hand-copies descriptions back out, once per client. The aggregate tool hands them a
   number and then sends them to fetch the meaning by hand. **This is now the committed problem
   statement for the build.**

6. **The owner dimension — a landlord is invisible; only buildings are visible.** New,
   identified 9 August 2026. The record is published per building, and the linking information
   (registered owner and officer names) sits in a *different* HPD dataset than the violations.
   A landlord operating twelve buildings therefore appears as twelve unrelated records. Tenant
   organisers and housing reporters rebuild the portfolio by hand each time, and it goes stale
   immediately. Confidence is lower than items 1–5: the workflow is inferred from the landscape
   rather than observed, and needs primary contact before it is relied on.

### Gaps in our own build, measured

Not aspirational. These are known, with numbers:

- **Coverage.** 250 buildings in one community district. The baked-artifact architecture holds
  to roughly 40,000 buildings before it exceeds the 256 MB VM; all 180,000 HPD-registered
  multifamily buildings would need ~914 MB and a different storage design.
- **Violation descriptions.** HPD publishes them; our ingest does not fetch them, so the agent
  can say *how many* violations exist but not *what they are*. Concretely,
  `crates/model/src/lib.rs` defines `Violation { class, open, year }` — there is nowhere for a
  description to go, so this is a schema change and an ingest change, not just a fetch.
- **No owner dimension.** We key on BBL throughout. Two buildings owned by the same landlord
  have no relationship in our data, because we never ingest HPD's registration/contacts
  dataset. Recorded as a gap; not committed.
- **Class I violations excluded.** 753 records skipped on the curated set — now stated on the
  card and in `/meta`, but not yet scored.
- **No refresh.** The artifact is a point-in-time snapshot. Nothing re-ingests on a schedule.

---

## 5. The market figure not to use

Global proptech, 2025, by research firm:

| Firm | 2025 size | Forecast |
|---|---|---|
| Fortune Business Insights | $40.19B | $104.57B by 2034 (11.9% CAGR) |
| SNS Insider | $43.0B | — |
| Grand View Research | $45.1B | — |
| Market.us | $45.7B | $178.5B by 2035 (14.6% CAGR) |
| Precedence Research | $47.08B | $185.31B by 2034 (16.4% CAGR) |

**It disagrees with itself.** A ~17% spread across five firms measuring the same year is a
signal about the reliability of the category, not a range to average.

**It is not our market.** "PropTech" bundles building management systems, virtual tours,
transaction tooling and investor analytics. The renter-facing building-record slice is a
rounding error inside it.

### What the bottom-up model says instead

Against verified denominators — **91,918** NYC multifamily buildings (REBNY, *Data Over
Rhetoric*, Feb 2026) and **2,357,000** total housing units (2023 NYCHVS):

| Adoption of 91,918 buildings | Avg price/building/yr | NYC ARR |
|---|---|---|
| 1% (919) | $300 | ~$276K |
| 5% (4,596) | $400 | ~$1.8M |
| 10% (9,192) | $500 | ~$4.6M |

Even optimistic adoption lands in **low single-digit millions of ARR in NYC**. Extending to
comparable metros might multiply that by 5–10×, but each new market needs its own ingest
pipeline against a different city's data schema — the cost scales *with* the revenue rather
than beneath it.

This is a real business. On this model alone it is not a venture-scale one — which is exactly
what Rentlogic's twelve years at nine people demonstrates.

---

## 6. The signal worth naming

Everything above points at one pattern, and it is the most interesting thing in this document
because it is structural rather than circumstantial.

> **Willingness to pay runs opposite to alignment.**
>
> The closer a buyer's interest is to the tenant's, the less they will pay for this data. The
> further away it is, the more.

The evidence is already in the tables above; laid end to end it is monotonic:

| Buyer | Alignment with the tenant | Will pay | What the money would be for |
|---|---|---|---|
| **Tenant lawyers, organisers, court staff** | Highest — they act *for* the tenant | ~nothing (grant-funded) | Daily case and campaign work |
| **Renters** | Perfect — it is about them | ~nothing; both comparables give it away | One decision every few years |
| **Landlords** | Opposed on the thing that matters | Proven (Rentlogic) | Advertising a *good* grade |
| **Brokers** | Aligned with *closing*, not with the tenant | Moderate (Openigloo's pivot) | Filling a unit |
| **Insurers** | Inverted | Highest of all | Habitability exclusions — denying a claim |

Read the last column downward. The willingness to pay rises exactly as the purpose rotates
away from the person the record describes. At the top, the use is "help this tenant." At the
bottom, it is "use this building's condition against the tenant living in it."

### Why this is the answer rather than one of the others

Four other signals in this document are real:

- Every layer of the landscape has a settled payer except this one (§2b).
- The in-market moment is 1 in 75, so reach was never Openigloo's constraint (§3).
- The highest-frequency users have the lowest budget (§3).
- Nobody turns violation *counts* into violation *meaning* (§4).

The first three are **consequences of this one.** The layer has no settled payer because every
available payer is somewhere down the gradient. The high-frequency, zero-budget worker sits at
the top of it. Twelve years and two companies did not fail to find a business model; they found
that the models which work require pointing the data away from the person it was collected
about. Rentlogic went to landlords and inherited adverse selection — the worst buildings never
buy. Openigloo went to the transaction. Neither of those is a mistake. They are what the
gradient permits.

The fourth — violation meaning — is a genuine product gap and not part of this pattern, which
is why it is worth keeping separate rather than folding in.

### Where the pattern might break, which is what makes it interesting rather than just bleak

Two buyers in §4 do not obviously sit on the gradient:

- **Lenders** pricing multifamily credit risk. A lender wants the loan priced correctly. A
  building with a bad violation history is a genuine default signal. Nothing about that
  requires harming the tenant.
- **City agencies** targeting enforcement. The enforcement body already holds the data; the
  value would be prioritisation, and its purpose is the same as ours.

Whether those two are real exceptions or just less obvious points on the same line is the
question this research does not answer.

**Named, not solved.** Deciding what to do about it is the next block's work, and any answer
that ignores the gradient is going to rediscover it the expensive way.

---

## 7. Open questions — the live edge of this document

1. **Current pricing.** Rentlogic's public numbers are from 2018. Openigloo's subscription
   tiers and brokerage fee are described qualitatively, not published. Both need direct contact.
2. **Openigloo's real revenue.** The $1.37M figure is a scraped estimate.
3. **Rent-burden reconciliation.** Our case study cites **51.6%** rent-burdened (RGB 2026
   Income & Affordability Study); the 2023 NYCHVS reports **29.5%**. Very likely different
   populations or thresholds — renter households versus all households — rather than a
   contradiction, but the distinction must be established before either is cited in front of
   someone who knows the other.
4. **Whether the white space in §4 is actually empty**, or merely not visible from public
   search.
5. **Whether renters use it at the moment that matters.** Openigloo reached 3M+ people and
   still had to leave for brokerage revenue. Audience was not the constraint. This is the
   question the comparables leave open and the one our own build cannot yet answer.

---

## Sources

- Openigloo pivot to brokerage — *The Real Deal*, 28 Aug 2025 — https://therealdeal.com/new-york/2025/08/28/openigloo-won-over-tenants-can-it-do-the-same-with-landlords/
- Openigloo — https://www.openigloo.com/
- Openigloo company profile — https://www.crunchbase.com/organization/openigloo
- Rentlogic raises $2.4M — *TechCrunch*, 3 Aug 2018 — https://techcrunch.com/2018/08/03/rentlogic-lands-millions-to-grade-nyc-real-estate-for-renters-and-landlords/
- Rentlogic expansion and pricing — *Inman*, 7 Aug 2018 — https://www.inman.com/2018/08/07/nyc-building-grading-startup-rentlogic-raises-2-point-4-m-for-expansion-to-other-cities/
- Rentlogic per-unit pricing — *Techweek* — https://techweek.com/rentlogic-building-certification-newyork-startup/
- NYC vacancy rate at 1.4% — NYC HPD — https://www.nyc.gov/site/hpd/news/007-24/new-york-city-s-vacancy-rate-reaches-historic-low-1-4-percent-demanding-urgent-action-new
- NYC Housing and Vacancy Survey — Rent Guidelines Board — https://rentguidelinesboard.cityofnewyork.us/research/nyc-housing-vacancy-survey/
- Moody's to acquire CAPE Analytics — Moody's IR, 13 Jan 2025 — https://ir.moodys.com/press-releases/news-details/2025/Moodys-to-Acquire-CAPE-Analytics-Adding-AI-Powered-Geospatial-Property-Risk-Intelligence-to-Its-Industry-Leading-Insurance-Risk-Models/default.aspx
- Moody's acquires CAPE Analytics — *Insurance Journal*, 13 Jan 2025 — https://www.insurancejournal.com/news/national/2025/01/13/807956.htm
- ZestyAI — https://zesty.ai/
- Verisk BuildFax permit data for insurers — https://www.globenewswire.com/news-release/2021/02/17/2177301/0/en/An-Industry-First-Canadian-Property-Insurers-Can-Unearth-Insights-from-Building-Permits-with-Verisk-s-BuildFax.html
- Housing code violations and insurer habitability exclusions — *Distinguished* — https://distinguished.com/blog/housing-code-violations-in-los-angeles-an-interview-with-katie-vespia/
- PropTech market size — Fortune Business Insights — https://www.fortunebusinessinsights.com/proptech-market-108634
- PropTech market size — Grand View Research — https://www.grandviewresearch.com/industry-analysis/proptech-market-report
- PropTech market size — Precedence Research — https://www.precedenceresearch.com/proptech-market
- PropTech market size — Market.us — https://market.us/report/proptech-market/
- PropTech market size — SNS Insider — https://www.snsinsider.com/reports/proptech-market-6857
