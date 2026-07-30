# HouseCheck — Market Research

**Researched:** 2026-07-29 · **Standard:** every figure carries its source, and
confidence is stated where a source is weak. Nothing here is estimated and then
presented as measured.

---

## Executive summary

Two companies have already built close variants of HouseCheck in New York, and
**neither monetised the building-data product itself.** Rentlogic sells
certification badges to landlords and is still nine people after twelve years.
Openigloo used the data to build an audience of millions, then pivoted into
brokerage, where the revenue actually is.

That convergence is the most useful finding in this document. The scoring engine
is a customer-acquisition layer, not the product.

The global "proptech" market figure — $40–47B depending on which research firm
you ask — is the wrong number to quote, and is not used here. A bottom-up model
against verified NYC building counts puts the realistic ceiling on the data
product at **low single-digit millions of ARR in NYC**, consistent with what the
comparables have actually raised and built.

There is real money in property risk data sold to insurers, evidenced by a 2025
acquisition by Moody's. It is a different dataset than ours, and the one buyer
whose needs match ours exactly wants it for a purpose that inverts our mission.
That is discussed in §5 rather than buried.

---

## 1. Direct comparables

### Rentlogic

The closest analogue to HouseCheck that exists.

| | |
|---|---|
| Founded | 2013, New York City |
| Team | 9 employees (2025) |
| Stage | Seed |
| Product | Grades every NYC apartment building **A / B / C / F** on health and safety, scoring ~150 variables from city agency data plus physical inspections |
| Renters | Free |
| Revenue | Landlords buy certification, signage and marketing: **$99–$1,499 per building**, varying by size — roughly **20¢ per apartment unit per month** (~$2.40/unit/yr) |
| Funding | **$2.4M seed** (2018) — Urban-X, Urban.Us, Kairos, Edgar Bronfman Jr. |

**Read:** the model works and it is small. Twelve years, nine people, still
seed-stage. Published pricing is 2018-era; nothing current is public, so treat
the per-unit figure as a floor-level indication rather than today's rate card.

**Structural problem worth naming:** a landlord buys the badge to advertise a
*good* grade. The buildings with the worst records — the ones a renter most
needs warned about — will never be customers. Revenue concentrates among the
landlords who least need scrutiny.

### Openigloo

| | |
|---|---|
| Founded | 2020, Brooklyn. CEO Allia Mohamed |
| Reach | States it has supported **3M+ NYC renters** |
| Product | Tenant reviews plus city data — violations, bedbug complaints, eviction filings, litigation history; building scores for maintenance, heat and water pressure, pests, landlord responsiveness |
| Investors | Trend Forward Capital, Gutter Capital, MetroCap Partners |

**The pivot.** In August 2025 *The Real Deal* published "Openigloo won over
tenants. Can it do the same with landlords?" The company now runs two revenue
lines:

1. **Renter subscriptions** — tiered access to full reviews and building reports.
2. **Brokerage** — exclusive agreements to list a landlord's portfolio, then
   parsing applications, verifying income, running credit, and **guaranteeing
   the rent**. Pricing is dynamic; a typical fee starts near **half of one
   month's rent**.

A third-party aggregator estimates annual revenue near $1.37M. That is a scraped
estimate with no disclosed methodology — **low confidence, do not cite it
externally.**

### What the two of them tell us

Two independent teams, seven years apart, built a renter-facing building-data
product in the same city. Neither made the data itself the business. One sells
badges to the landlords being graded; the other moved to where a transaction
happens and money changes hands.

If HouseCheck pursues revenue, the evidence says the data buys distribution and
something else buys the revenue.

---

## 2. The market figure not to use

Global proptech, 2025, by research firm:

| Firm | 2025 size | Forecast |
|---|---|---|
| Fortune Business Insights | $40.19B | $104.57B by 2034 (11.9% CAGR) |
| SNS Insider | $43.0B | — |
| Grand View Research | $45.1B | — |
| Market.us | $45.7B | $178.5B by 2035 (14.6% CAGR) |
| Precedence Research | $47.08B | $185.31B by 2034 (16.4% CAGR) |

Two reasons this is the wrong number for us.

**It disagrees with itself.** A ~17% spread across five firms measuring the same
year is a signal about the reliability of the category, not a range to average.

**It is not our market.** "PropTech" bundles building management systems,
virtual tours, transaction tooling and investor analytics. The renter-facing
building-record slice is a rounding error inside it. Quoting $45B to describe
HouseCheck's opportunity would be the same failure as the unsourced
"$3.6B property-data market" line that was removed from the case study — a big
number doing rhetorical work its source cannot support.

---

## 3. Bottom-up market sizing

Built on figures already verified in
[`HouseCheck_Claim_Verification_Dossier.md`](../HouseCheck_Claim_Verification_Dossier.md)
and the NYC Housing and Vacancy Survey.

**Denominators**

- **91,918** multifamily buildings in NYC — REBNY, *Data Over Rhetoric*
  (Feb 2026). This is the B2B denominator: the buildings a landlord product
  could be sold against.
- **2,357,000** total housing units in NYC — 2023 NYCHVS.
- **1.41%** rental vacancy in 2023, the lowest since 1968; only **33,210** units
  available citywide.
- Roughly **1M** rent-stabilised or rent-controlled apartments, about **2.5M**
  tenants.

**Landlord-certification model**, priced against Rentlogic's public card:

| Adoption of 91,918 buildings | Avg price/building/yr | NYC ARR |
|---|---|---|
| 1% (919) | $300 | ~$276K |
| 5% (4,596) | $400 | ~$1.8M |
| 10% (9,192) | $500 | ~$4.6M |

**Interpretation.** Even optimistic adoption lands in low single-digit millions
of ARR in NYC. Extending to comparable metros might multiply that by 5–10×, but
each new market needs its own ingest pipeline against a different city's data
schema — the cost scales with the revenue rather than beneath it.

This is a real business. It is not, on this model alone, a venture-scale one —
which is exactly what Rentlogic's twelve years at nine people demonstrates.

---

## 4. Customer segments, ranked by evidence

| Segment | Reach | Willingness to pay | Evidence |
|---|---|---|---|
| **Renters** | ~2.5M NYC tenants; Openigloo claims 3M+ reached | Very low | Both comparables give the data away free. Openigloo charges for *depth*, not access |
| **Landlords / property managers** | 91,918 multifamily buildings | Low–moderate | Proven: Rentlogic's entire revenue line. Adverse selection applies (§1) |
| **Brokers / leasing** | — | Moderate | Openigloo's pivot. The FARE Act (eff. June 2025) moved broker fees to landlords, changing who buys tools that close a lease |
| **Legal services, tenant orgs, city agencies** | — | Low, but grant-funded | Aligned with the referral directory already shipped. Credibility over revenue |
| **Insurers / lenders / investors** | — | Highest | Real exits exist (§5), but a different dataset and a mission conflict |

---

## 5. Institutional risk data — real money, wrong shape

**There are genuine outcomes in property risk data.**

- **Cape Analytics** entered an agreement to be acquired by **Moody's**,
  announced 13 January 2025, expected to close in Q1 2025. Price undisclosed;
  Moody's stated the transaction was not expected to have a material impact on
  its financial results, which bounds the size.
- **ZestyAI** sells property-level risk to P&C insurers and regulators, and has
  added a former Verisk CEO to its board.
- **Verisk / BuildFax** sells building-permit-derived property intelligence to
  insurers.

**But they sell a different product.** Every one of these models physical and
catastrophe risk — roof condition, vegetation, wildfire, hail, wind, water —
derived largely from aerial and satellite imagery. None of them sells
housing-code violations, habitability, or landlord behaviour.

**Searching specifically for a vendor selling code-violation or landlord-risk
scores to insurers or lenders returned no established player.** That is genuine
white space.

### The conflict to decide deliberately

The same research surfaced why that space may be open. Insurers are beginning to
explore **habitability exclusions** — provisions allowing a claim to be denied
where a property is uninhabitable due to code violations or deferred
maintenance.

That is the natural buyer for precisely HouseCheck's dataset. It is also the
exact inversion of the product's purpose. HouseCheck exists so a renter knows
what they are signing into. Sold to that buyer, the same records help an insurer
refuse a claim on the same bad building, and the party harmed is the tenant.

This is recorded here rather than omitted because it is the highest
willingness-to-pay path found, and a project that publishes its own corrections
should make that call on purpose.

**Cleaner variants of the same path**, where the buyer's incentive points the
same direction as ours:

- **Lenders** pricing multifamily credit risk — a building's violation history
  is a real default signal, and the lender's interest in an accurately-priced
  loan does not require harming a tenant.
- **City agencies** targeting enforcement — the enforcement body already has the
  data; the value is prioritisation.

---

## 6. Open questions

Not answered by this round of research:

1. **Current pricing.** Rentlogic's public numbers are from 2018. Openigloo's
   subscription tiers and brokerage fee are described qualitatively, not
   published. Both would need direct contact.
2. **Openigloo's real revenue.** The $1.37M figure is a scraped estimate.
3. **Rent-burden reconciliation.** The case study cites **51.6%** rent-burdened
   (RGB 2026 Income & Affordability Study). The 2023 NYCHVS reports **29.5%**.
   These are very likely different populations or thresholds — renter households
   versus all households — rather than a contradiction, but the distinction
   should be established before either is cited in front of someone who knows
   the other.
4. **Whether the white space in §5 is actually empty**, or merely not visible
   from public search. Absence of evidence is not evidence of absence, and this
   document should not be read as claiming the field is clear.

---

## Sources

- Openigloo pivot to brokerage — *The Real Deal*, 28 Aug 2025 — https://therealdeal.com/new-york/2025/08/28/openigloo-won-over-tenants-can-it-do-the-same-with-landlords/
- Openigloo — https://www.openigloo.com/
- Openigloo company profile — https://www.crunchbase.com/organization/openigloo
- Rentlogic raises $2.4M — *TechCrunch*, 3 Aug 2018 — https://techcrunch.com/2018/08/03/rentlogic-lands-millions-to-grade-nyc-real-estate-for-renters-and-landlords/
- Rentlogic expansion and pricing — *Inman*, 7 Aug 2018 — https://www.inman.com/2018/08/07/nyc-building-grading-startup-rentlogic-raises-2-point-4-m-for-expansion-to-other-cities/
- Rentlogic per-unit pricing — *Techweek* — https://techweek.com/rentlogic-building-certification-newyork-startup/
- Rentlogic company profile — https://tracxn.com/d/companies/rentlogic/__lKB31oNmdVQfEMD56oekQrkAUBLCD0KXDVHFe5W5Apc
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
