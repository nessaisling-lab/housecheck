# Design Session — Address → BBL at City Scale

**The question:** within one community district, resolving a typed address to a building works. At
222,433 buildings it was flagged as the biggest unmeasured risk, because a geocoding failure
becomes a silent coverage gap rather than a visible error. Does the current approach survive?

**Answer: no, in four separate ways.** Three are cheap to fix and one is a real design change.

**Dated:** 2026-08-09. **Status:** findings and recommendation, not yet built.

---

## How it works today

`GET /search` runs `search_curated()`, which:

1. normalises the query with `normalize_address()`
2. loads **every** building, normalises each stored address, and keeps those where the stored
   address equals, starts with, or contains the query
3. ranks exact > prefix > substring
4. if nothing matches, falls back to **NYC GeoSearch** (`geosearch.planninglabs.nyc`), which is
   NYC Planning Labs' official geocoder — free, no key, and it returns a BBL

That fallback is the strongest thing in the current design and the reason this is fixable rather
than a rewrite.

---

## Finding 1 — the normaliser is wrong on real NYC street names

`normalize_address()` expands abbreviations by matching **any** token, wherever it appears. Two
of those rules corrupt addresses that genuinely exist:

| Rule | Corrupts | Real PLUTO lots affected |
|---|---|---:|
| `ST` → `STREET` | `ST NICHOLAS AVENUE` → `STREET NICHOLAS AVENUE` | **167** |
| `W` → `WEST` | `AVENUE W` → `AVENUE WEST` | **403** |
| `N` → `NORTH` | `AVENUE N` → `AVENUE NORTH` | **744** |

Saint-prefixed streets (St Nicholas, St Marks, St Johns) and Brooklyn's lettered avenues are not
edge cases — they are ordinary Brooklyn and Manhattan addresses, and every one of them is
currently unfindable by its real name.

**Fix:** expand by **position**, not by presence.

- A street-type token (`ST`, `AVE`, `BLVD`, …) only expands in **final** position, and never when
  it is the **first** token — `ST NICHOLAS` keeps `ST` as Saint.
- A directional (`N`, `S`, `E`, `W`) only expands in **leading** position — `W 42 STREET` becomes
  `WEST 42 STREET`, while `AVENUE W` is left alone.

Cheap, and wrong at any scale — worth doing before citywide regardless of everything below.

## Finding 2 — a linear scan does not survive 222,433 buildings

`search_curated` loads every building and normalises every stored address **per query**. At 250
buildings that is invisible. At 222,433, normalising on every keystroke is roughly two orders of
magnitude past the 2.2 ms the whole card currently takes.

**Fix:** normalise **once, at ingest**, and store the normalised form in an index. Serving does an
index lookup, never a scan. SQLite ships FTS5, which handles prefix matching for type-ahead
without loading the table.

This is the same principle the rest of the architecture already runs on: expensive work happens
once, at ingest, on a laptop.

## Finding 3 — the address string is not unique, and substring matching hides it

| | |
|---|---:|
| PLUTO lots | 858,602 |
| Distinct address strings | 828,567 |
| **Collisions** | **30,035 — 3.5%** |

3.5% of lots share an address string with another lot. Substring matching makes this worse:
`FULTON STREET` matches every building on Fulton Street in every borough.

**It is already happening at 250 buildings.** Our own curated set has 250 buildings and **249
distinct addresses** — `FULTON STREET`, with no house number at all, appears twice against two
different BBLs. PLUTO's address field is sometimes just a street name.

**Fix:** the index key must include the **borough**, results must be disambiguated by borough in
the UI, and an address with no house number should be treated as unresolvable rather than matched.

## Finding 4 — PLUTO holds one address per lot, and buildings have several

PLUTO's `address` is a single primary address. Corner buildings, buildings with multiple entrances
and buildings addressed on a different street from their lot frontage all have alternates that a
user will reasonably type and that will never match.

The authoritative source for *every* address on a BBL is the **Property Address Directory**,
catalogue id `bc8t-ecyu`. **Not yet verified:** it did not respond on the Socrata tabular resource
endpoint, which suggests it is distributed as a file download rather than a queryable dataset.
That has to be confirmed before it is planned around — this document has already had two dataset
ids turn out not to resolve, and guessing a third would be the same mistake.

---

## Recommendation

**Build the index at ingest from an authoritative address source; keep GeoSearch as the runtime
fallback.**

1. **Fix the normaliser** — positional expansion. Independent of everything else.
2. **At ingest**, expand every BBL to all of its known addresses (PAD, once verified), normalise
   each, and write an `address_index(normalised, borough, bbl)` table with an FTS5 index.
3. **At serve**, look up the index. No scan, no per-query normalisation of the corpus.
4. **Keep GeoSearch** for anything the index misses — typos, brand-new buildings, addresses the
   city knows and we have not ingested. It is already wired, already free, and already
   authoritative.
5. **Never silently fail.** An address the index cannot resolve and GeoSearch cannot resolve is
   "we could not find that address", and an address GeoSearch resolves to a BBL we do not hold is
   "that building is outside our coverage" — two different messages. The current handler already
   makes this distinction; it must survive the rewrite, because a silent geocoding gap is
   indistinguishable from a clean building.

### Why not geocode at request time only

GeoSearch could be the primary path and skip the index entirely. Rejected: it puts a network round
trip in front of every search, makes type-ahead impossible, and makes an outage look like an empty
city. As a fallback it is excellent; as the hot path it trades a measured 2.2 ms for someone
else's uptime.

### Why not run NYC Geosupport locally at serve time

Geosupport is the city's own geocoding engine and is authoritative, but it carries a large native
library and data files. Correct use is **at ingest**, to build the index — where its weight costs
nothing because it never ships in the image.

---

## Open items

- **Verify `bc8t-ecyu`** is obtainable and what it actually contains.
- **Measure the index size.** All addresses for 222,433 buildings is more rows than buildings —
  possibly several million — and it lands in the same artifact as everything else.
- **Decide the type-ahead contract citywide.** Prefix search over 222k buildings returns very
  different result sets than over 250, and ranking by borough or proximity is a product decision,
  not just an index one.
- **Measure geocoding coverage after a real borough ingest** — what share of addresses resolve,
  and what the failures look like.
