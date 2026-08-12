# HouseCheck — Claim Verification Dossier

**Date:** 2026-07-24
**Standard:** true, not merely persuasive. Concessions stated plainly; uncertainty marked, not hidden.
**Crux claims re-verified firsthand** (direct WebFetch of the primary URLs on 2026-07-24).

---

## Executive summary

Both peer challenges are **substantively correct**, and conceding them makes the project *stronger*.
- **Claim I (761,352 / "~11% Class C") — REAL.** Verbatim in a genuine REBNY report, *Data Over Rhetoric* (REBNY Research, Feb 22 2026). The presentation's "fabricated" framing was wrong (an absence-of-evidence error against a report published after the original check). Confirmed by direct fetch of the REBNY page + independent corroboration in *The Real Deal*.
- **Claim II (HPD `bbl` column) — the presentation was WRONG; retract.** `wvxf-dwi5` has a native `bbl` column (schema position 40 of 41), confirmed three independent ways (schema metadata, live API values, CSV export header) and firsthand in our own live ingest run.
- **Everything else checks out** as Confirmed or with minor labeling nuances. The only other outright data-stack error is a stale "311 = 2010–present" label (it is **2020–present**).

Integrity posture: **strong**. Two honest corrections; one of them *restores* a real, citable statistic.

---

## Disputed Claim I — "761,352 buildings" and "~11% Class C"

**Verdict: REAL and correctly attributed to REBNY. The peer is right.**

Verified verbatim on the REBNY page (fetched 2026-07-24):
- *"We analyze 761,352 residential buildings at the tax-lot level (BBL)…"*
- *"89% of the 761,352 buildings have no Class C violations over the past 24 months."*  → ~11% have ≥1 Class C.
- *"In the multifamily universe of 91,918 buildings only, 10% of buildings account for 80% of evictions and 50% of violations over the past 24 months."*

**Three distinct numbers, kept separate:**
1. **761,352** = full residential universe analyzed at the tax-lot (BBL) level.
2. **~11% Class C** = arithmetic complement of "89% have no Class C." (Class C = a violation *severity*, not a building class — phrase carefully; do not say "11% of buildings are Class C.")
3. **10% → 80% / 50%** = the *multifamily subset* of 91,918 buildings only, not the full universe.

**Source:** https://www.rebny.com/reports/data-over-rhetoric-a-closer-look-at-housing-violations-evictions/
**Corroboration:** *The Real Deal*, Lilah Burke, 26 Feb 2026 — https://therealdeal.com/new-york/2026/02/26/evictions-violations-concentrated-in-10-of-nyc-housing-stock/

**Own it:** the "fabricated" call originated in HouseCheck's own fact-check; it was wrong because the report post-dates that check. We didn't invent a number — we wrongly deleted a real one.

---

## Disputed Claim II — HPD `bbl` column (`wvxf-dwi5`)

**Verdict: The dataset DOES have a native `bbl` column. The presentation was WRONG. Retract the "we reconstruct BBL" claim.**

Native `bbl` = column **40 of 41** in the official Socrata schema (`fieldName: "bbl", name: "BBL", dataTypeName: "text"` — the 10-digit Borough-Block-Lot identifier). Verified three ways:
1. **Schema metadata** — https://data.cityofnewyork.us/api/views/wvxf-dwi5.json (bbl at position 40)
2. **Live API values** — `$where=bbl IS NOT NULL` returns valid, correctly-decomposing BBLs (e.g. block 4654, lot 21 → `bbl` 2046540021).
3. **CSV export header** — ends `…,censustract,bin,bbl,nta` (seen firsthand in our own live run, 2026-07-24).

**Root cause (so it can't repeat):** a single old row was likely probed via `/resource/wvxf-dwi5.json?$limit=1`. Socrata omits null-valued keys from JSON, and pre-2013 records predate BBL geocoding (empty `bbl`), so the key was absent from that one row. A data-vintage artifact, not a schema fact. Our reconstruction (`boro + zpad5(block) + zpad4(lot)`) happens to match the native value, so **no incorrect data shipped** — the code was just redundant.

**Dataset page:** https://data.cityofnewyork.us/Housing-Development/Housing-Maintenance-Code-Violations/wvxf-dwi5

---

## Every other headline claim

| Claim | Verdict | What's true | Source |
|---|---|---|---|
| 51.6% rent-burdened / 28.8% severe | Nuanced (label) | Both exact; 28.8% (50%+) is a *subset* of the 51.6% (30%+). Drop "moderately." | RGB 2026 Income & Affordability Study |
| ~$1,761 stay-vs-move gap | Confirmed | Real, but **Realtor.com** (Q1 2026), industry data — not a government stat. | prnewswire.com (Apr 28 2026) |
| ~11.1M HPD violation records | Confirmed | Live count ≈ 11,126,984 (2026-07-24). All records ever issued; label "total historical, as of {date}." | wvxf-dwi5 `$select=count(*)` |
| FARE Act — eff. June 11, 2025 | Confirmed | NYC DCWP: "FARE Act Is Now in Effect." | nyc.gov/site/dca/news/018-25 |
| Good Cause Eviction — Apr 20, 2024 | Confirmed | NYC effective date; add "in NYC; opt-in elsewhere in NY." | nyc.gov/site/hpd/.../good-cause-eviction.page |
| HUD FMR FY2026 (NY metro) | Confirmed | Exists (Fed Reg 22 Aug 2025, rev. 21 Apr 2026); pull per-BR live via HUD API. | federalregister.gov/.../2025-16060 |
| 311 = "2010–present" | **Corrected** | Coverage is **2020–present** (~21.9M rows); 2010–19 in `76ig-c548`. | data.cityofnewyork.us/d/erm2-nwe9 |
| PLUTO / MapPLUTO | Nuanced | `64uk-42ks` is **PLUTO tabular**, not MapPLUTO (geospatial). | data.cityofnewyork.us/d/64uk-42ks |
| All dataset IDs live & correct | Confirmed | wvxf-dwi5, erm2-nwe9, 43nn-pn8j, e5aq-a4j2, 39hk-dx4f (data.ny.gov), ufzp-rrqu all resolve. | NYC Open Data / data.ny.gov |

---

## Corrections adopted (exact wording)

1. **761,352 / Class C** — "REBNY analyzes **761,352 NYC residential buildings** at the tax-lot level. Per REBNY, **89% have no HPD Class C (immediately hazardous) violations** over 24 months — i.e. **~11% have at least one.** Within the multifamily subset (**91,918 buildings**), **10% account for 80% of evictions and 50% of violations.** Source: REBNY, *Data Over Rhetoric* (Feb 22 2026)." — do NOT say "11% of buildings are Class C."
2. **HPD BBL** — "HPD's `wvxf-dwi5` provides a native 10-digit **`bbl`** column (schema position 40), read directly. Older pre-geocoding records may have an empty BBL."
3. **Rent burden** — "**51.6% of NYC renter households are rent-burdened (30%+ of income), including 28.8% severely burdened (50%+).**" (drop "moderately")
4. **311** — "**311 Service Requests, 2020–present (`erm2-nwe9`), ~21.9M records;** 2010–19 in `76ig-c548`."
5. **PLUTO** — "**PLUTO (tabular, `64uk-42ks`).**"
6. **Good Cause** — "**effective April 20, 2024 (in NYC; opt-in elsewhere in NY State).**"
7. **11.1M HPD** — "**total historical violation records, as of {date}**."
8. **$1,761 gap** — attribute to "**Realtor.com, Q1 2026 rental data.**"

---

## Note on process

Two of my own research passes disagreed on Claim I (one concluded "fabricated," one "real"); the conflict was resolved — and then re-confirmed by a fresh, independent direct fetch of the REBNY URL — in favor of **real**. The "fabricated" note is formally overturned. This dossier concedes what is wrong on our side because doing so is both correct and the strongest possible defense of the project's integrity.
