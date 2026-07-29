# HouseCheck API contract

Backend for the HouseCheck frontend (Anthony's map/list + building Health Card). All responses
are JSON except `/health`. The server reads a bundled SQLite DB built by the ingest crate; scores
are computed from that snapshot.

- **Base URL (local dev):** `http://127.0.0.1:8787`
- **Host/port:** overridable via `HOST` / `PORT` env vars (container listens on `0.0.0.0:$PORT`).
- **CORS:** env-driven via `CORS_ALLOWED_ORIGIN` — permissive for local dev, restricted to one
  origin in prod (see [Environment variables](#environment-variables)).
- **Rate limiting:** a `ConcurrencyLimitLayer(64)` bounds in-flight requests (see note at bottom).
- **BBL:** the 10-digit NYC Borough-Block-Lot identifier, as a string (e.g. `"3018420001"`).

The curated MVP set is Brooklyn Community District 303 (Bed-Stuy), ~250 buildings.

---

## `GET /health`

Liveness probe.

- **Params:** none
- **Success:** `200 OK`, body is the literal text `ok` (`text/plain`, not JSON).

```bash
curl -s http://127.0.0.1:8787/health
```
```
ok
```

---

## `GET /building/{bbl}`

Full building **Health Card**: the building record, the 0–100 score breakdown, open-violation
counts, an accessibility likelihood label, and the honest rent-stabilization signal.

- **Path param:** `bbl` — 10-digit BBL string.
- **Success:** `200 OK` with the Health Card object below.
- **Errors:** `404 Not Found` if the BBL is not in the curated DB; `500` on an internal DB error.

```bash
curl -s http://127.0.0.1:8787/building/3018420001
```
```json
{
  "building": {
    "bbl": "3018420001",
    "address": "123 MACON STREET",
    "year_built": 1910,
    "num_floors": 3,
    "units_res": 6,
    "tract_geoid": "36047025300",
    "rent_stabilized": null,
    "rent_stab_units": null,
    "good_cause": false,
    "has_elevator": false,
    "near_ada_subway_m": 420,
    "complaints_311": 37,
    "latitude": 40.6829,
    "longitude": -73.9251,
    "restaurant_grade": "A"
  },
  "score": {
    "total": 72,
    "condition": 79,
    "legal": 60,
    "neighborhood": 100,
    "accessibility": 30
  },
  "open_violations": { "a": 0, "b": 1, "c": 1 },
  "access_likelihood": "Lower",
  "stabilization": {
    "status": "unverified",
    "message": "Unverified — no DOF stabilization record found for this building."
  }
}
```

Field notes:

- `building.latitude` / `building.longitude` — building centroid from PLUTO (`null` if it was
  never geocoded). Same coordinates the `/buildings` map feed uses.
- `building.restaurant_grade` — letter grade (`"A"`/`"B"`/`"C"`) of the nearest DOHMH-graded
  restaurant within ~200 m, or `null`. **Neighborhood context only — never part of any score.**
- `building.rent_stab_units` — count of rent-stabilized units on the latest NYC DOF
  Statement-of-Account record (2024). `>0` when stabilized, `0` when the building is on the DOF
  record with no stabilized units, `null` when no DOF record was found. Source below.
- `access_likelihood` — one of `"Higher"`, `"Mixed"`, `"Lower"`. A likelihood, not a certification.
- `stabilization` — three honest states derived from `building.rent_stabilized` +
  `building.rent_stab_units` (`N` is the unit count):
  | `rent_stabilized` | `status`         | `message` |
  |-------------------|------------------|-----------|
  | `true`            | `likely`         | `Likely rent-stabilized — N units on the latest NYC DOF record (2024). A signal, not a legal ruling; confirm with DHCR.` |
  | `false`           | `none_on_record` | `No stabilized units on the latest DOF record (2024) — public data lags, so not proof it is market-rate.` |
  | `null`            | `unverified`     | `Unverified — no DOF stabilization record found for this building.` |

  > **Source:** [JustFix.org](https://github.com/JustFixNYC/nyc-doffer) (`nyc-doffer`), derived from
  > NYC DOF Statement of Account records; latest year 2024. The count is the most recent non-blank
  > year in the JustFix dataset. It's an incomplete public signal, never a legal ruling — the
  > wording is intentionally hedged and never overstates a match.

---

## `GET /buildings`

Compact list/map feed for the frontend — every building in the curated set with its coordinates
and total score. The score is computed on the fly per row, so it stays in lockstep with
`/building/{bbl}`. Ordered by BBL.

- **Params:** none
- **Success:** `200 OK` with a JSON array of items.

```bash
curl -s http://127.0.0.1:8787/buildings
```
```json
[
  {
    "bbl": "3018420001",
    "address": "123 MACON STREET",
    "latitude": 40.6829,
    "longitude": -73.9251,
    "score": 72
  },
  {
    "bbl": "3018420015",
    "address": "45 HALSEY STREET",
    "latitude": 40.6841,
    "longitude": -73.9333,
    "score": 88
  }
]
```

---

## `GET /compare?bbls=<a,b,c>`

Side-by-side building comparison: builds the full **Health Card** (identical logic to
`/building/{bbl}`) for each requested BBL and returns them together, so the frontend can render a
comparison table without N round-trips.

- **Query param:** `bbls` — comma-separated list of BBL strings (required, non-empty). Capped at
  **4** buildings per request to bound work; extras are ignored. Duplicates are de-duplicated.
- **Success:** `200 OK` with `{ "buildings": [ <HealthCard>, ... ] }`. Cards are returned in the
  requested order.
- **Skipped BBLs:** any BBL **not** in the curated DB is **silently skipped** — it simply does not
  appear in `buildings` (so a mixed list of known/unknown BBLs still returns the known ones). Send
  `bbls` through `/search` first if you need to distinguish "not in set" from "typo".
- **Errors:** `400 Bad Request` if `bbls` is missing or empty (after trimming); `500` on an
  internal DB error.

```bash
curl -s 'http://127.0.0.1:8787/compare?bbls=3018420001,3018420015'
```
```json
{
  "buildings": [
    { "building": { "bbl": "3018420001", "rent_stab_units": null, "...": "..." }, "score": { "total": 72, "...": "..." }, "open_violations": { "a": 0, "b": 1, "c": 1 }, "access_likelihood": "Lower", "stabilization": { "status": "unverified", "message": "Unverified — no DOF stabilization record found for this building." } },
    { "building": { "bbl": "3018420015", "rent_stab_units": 24, "...": "..." }, "score": { "total": 88, "...": "..." }, "open_violations": { "a": 0, "b": 0, "c": 0 }, "access_likelihood": "Higher", "stabilization": { "status": "likely", "message": "Likely rent-stabilized — 24 units on the latest NYC DOF record (2024). A signal, not a legal ruling; confirm with DHCR." } }
  ]
}
```

Each element of `buildings` is exactly the object documented under [`GET /building/{bbl}`](#get-buildingbbl).

---

## `POST /rent-fairness`

Compare a user's monthly rent against two reference points: the Census tract median gross rent
(ACS B25064) and the current HUD Fair Market Rents by bedroom for the NYC metro area.

- **Body (JSON):** `{ "bbl": string, "monthly_rent": integer }` (`monthly_rent` must be > 0)
- **Success:** `200 OK` with the object below.
- **Errors:** `400 Bad Request` if `monthly_rent <= 0`; `404 Not Found` if the BBL is unknown or
  the tract has no reliable median; `500` on an internal DB error.

```bash
curl -s -X POST http://127.0.0.1:8787/rent-fairness \
  -H 'content-type: application/json' \
  -d '{"bbl":"3018420001","monthly_rent":3000}'
```
```json
{
  "bbl": "3018420001",
  "user_rent": 3000,
  "tract_median": 2580,
  "pct_vs_median": 16.28,
  "verdict": "16% above neighborhood median",
  "hud_fmr": {
    "area": "New York, NY HUD Metro FMR Area",
    "fiscal_year": 2026,
    "studio": 2529,
    "one_br": 2655,
    "two_br": 2910,
    "three_br": 3644
  }
}
```

Field notes:

- `tract_median` — Census ACS 5-year median gross rent for the building's tract, in whole dollars.
- `pct_vs_median` — signed percentage of `user_rent` vs `tract_median`.
- `verdict` — human summary vs the tract median (`"above"` / `"below"` / `"about at"`).
- `hud_fmr` — embedded FY2026 HUD Fair Market Rents (New York, NY HUD Metro FMR Area, which covers
  Kings County / Brooklyn), effective Oct 1, 2025 – Sep 30, 2026. Constants, no HUD API key. Lets
  the frontend show "vs HUD FMR" by bedroom next to the tract-median comparison.

---

## `GET /search?address=<text>`

Resolve free text to buildings, so the frontend can jump straight to one and tell whether it is in
the curated set.

**Two passes, in this order:**

1. **Our own rows.** The text is normalised (case, punctuation, and street-type/compass
   abbreviations — `464 Madison St` ≡ `464 MADISON STREET`) and matched against every stored
   address. Matches rank exact → prefix → substring, capped at 8. No network involved.
2. **NYC GeoSearch**, only when pass 1 finds nothing — to distinguish "a real address outside the
   pilot" from "not an address".

The order matters. Geocoding first meant a flaky upstream could veto a building we hold: NYC
GeoSearch is not deterministic, and the same query intermittently returns `502` or resolves to a
different building on the same street, so an address inside the pilot would sometimes report as out
of coverage.

- **Query param:** `address` — free text (required, non-blank).
- **Success:** `200 OK` with a **JSON array** of matches, best first. Always an array, whichever
  pass answered.
- **Errors:** `400 Bad Request` if `address` is missing/blank. If pass 1 finds nothing and GeoSearch
  also fails: `404 Not Found` (no match, or no BBL on the match) or `502 Bad Gateway` (upstream
  unreachable/unparseable). A curated match never reaches these.

```bash
curl -s 'http://127.0.0.1:8787/search?address=464%20Madison%20St'
```
```json
[
  {
    "bbl": "3018260029",
    "label": "464 MADISON STREET",
    "in_curated_set": true
  }
]
```

Field notes:

- `bbl` — the stored BBL for a curated match; otherwise the canonical 10-digit BBL from the
  GeoSearch feature (handles both `properties.addendum.pad.bbl` and `properties.pad_bbl`, string or
  number).
- `label` — our stored address for a curated match; GeoSearch's label otherwise.
- `in_curated_set` — `true` if that BBL exists in our DB (so `/building/{bbl}` will resolve).
  Always `true` for pass-1 results.

---

## `POST /summary`

**Optional** plain-language summary of a building's Health Card, generated by an LLM via
[OpenRouter](https://openrouter.ai/). Disabled unless the server has an `OPENROUTER_API_KEY`, so
the endpoint is safe to deploy without one.

- **Body (JSON):** `{ "bbl": string }`
- **Success:** `200 OK` with `{ "bbl": string, "summary": string }` — 2–3 plain-spoken sentences.
- **Errors:**
  - `404 Not Found` — the BBL isn't in the curated DB (checked **before** the LLM call).
  - `501 Not Implemented` — `OPENROUTER_API_KEY` is unset; body is
    `{ "error": "summary disabled — set OPENROUTER_API_KEY" }`. The endpoint is optional, so a
    missing key disables it rather than erroring the server.
  - `502 Bad Gateway` — the OpenRouter upstream failed, timed out (~20 s), or returned no content.
  - `500` — internal DB error building the card.

```bash
curl -s -X POST http://127.0.0.1:8787/summary \
  -H 'content-type: application/json' \
  -d '{"bbl":"3018420001"}'
```
```json
{
  "bbl": "3018420001",
  "summary": "This Bed-Stuy walk-up scores 72/100 overall, dragged down by one open serious (class-C) HPD violation, so ask the landlord what's being fixed. Rent-stabilization is unverified here, and with no elevator in a pre-FHA building, step-free access is unlikely. Neighborhood 311 volume is low, which is a good sign."
}
```

Implementation notes:

- Model: `nvidia/nemotron-3-ultra-550b-a55b:free` (a free OpenRouter model), called at the
  OpenAI-compatible endpoint `https://openrouter.ai/api/v1/chat/completions` with
  `Authorization: Bearer $OPENROUTER_API_KEY`.
- The system prompt instructs a "plain-spoken NYC renter's advocate" to be concrete and honest and
  **not invent facts**; the user message carries the card's key facts (score breakdown, open
  A/B/C violations, rent-stabilization signal, accessibility likelihood, nearby 311, and — since
  the request carries no user rent — the neighborhood tract median as rent context).

---

## `POST /agent/chat`

Multi-turn, grounded Q&A about one building. Conversation only — no tool calling yet
(slice 2 of `docs/agent/PRD-AGENT.md`).

```jsonc
// request
{
  "bbl": "3014800023",
  "messages": [                       // full history; server keeps the last 12 turns
    { "role": "user", "content": "are the violations here serious?" }
  ]
}
// 200
{
  "bbl": "3014800023",
  "answer": "…",
  "citations": ["NYC HPD violations (wvxf-dwi5)", "…"]   // only sources actually used
}
```

**Grounding.** The model receives a system prompt plus a delimited `BUILDING FACTS` block built
by the same function `/summary` uses, so the two endpoints can never answer from different facts
about the same building. The prompt states that content inside the block is data, never
instructions, and forbids legal advice, invented numbers, and speculation about individuals.
Client-supplied roles other than `assistant` are coerced to `user`, so a caller cannot inject a
second system turn.

**Tools.** The model may request data instead of answering directly. Six read-only tools are
offered — `get_building(bbl)`, `get_open_violations(bbl)`, `search_address(address)`, `legal_context(issue)`, `find_legal_help()`, `search_law(query)`.

`legal_context` returns published NY law for a housing problem with verifiable links, plus an
evidence checklist and the official complaint route. `find_legal_help` returns free tenant
legal services with phone numbers. `search_law` searches **only** an allowlist of
authoritative sources — nysenate.gov, law.cornell.edu, law.justia.com, nycourts.gov, nyc.gov,
hcr.ny.gov, lawhelpny.org, govinfo.gov, ecfr.gov — via OpenRouter's web plugin, and returns
titles, URLs and excerpts. The allowlist is a security control, not a quality filter: it is
what makes "web content is data, never instructions" realistic, and it keeps lead-generation
and scam sites out of an answer someone may read during a housing crisis. `search_law` uses a
separate small model (`OPENROUTER_SEARCH_MODEL`) so a lookup does not stack two slow
generations into one request.

The agent gives legal **information**, never advice, and never predicts an outcome — it has no
case history, no docket data, and has not seen the user's lease. It can draft a question the
user takes to a lawyer, in their own voice, citing the statute. **The model
never touches the database:** it asks, the server executes, the result is fed back. That
separation is what makes grounding enforceable rather than aspirational. The loop is capped at
**5 iterations**; hitting the cap returns `502` rather than looping on a billed call forever. A
hallucinated tool name is answered as data (`{"error": "unknown tool: …"}`) so the model can
recover instead of the request failing. `citations[]` grows only as tools actually succeed.

**Limits.** `max_tokens` 400 · last 12 turns · 2,000 characters per message · 30s upstream
timeout · 5 tool iterations · **10 requests per client per 60s**. The rate limit is a spend control, not just a load
control: this is the only endpoint that costs money per request. Client identity comes from
`Fly-Client-IP`, else the first hop of `X-Forwarded-For`.

| Status | Meaning |
|---|---|
| `400` | `messages` empty, or the last turn isn't a non-empty user turn |
| `404` | BBL not in the curated set — checked *before* the key, so probing is free |
| `413` | message longer than 2,000 characters |
| `429` | rate limit exceeded |
| `501` | `OPENROUTER_API_KEY` unset — the agent is optional |
| `502` | upstream call, decode, or empty completion |

## Environment variables

| Variable | Used by | Effect |
|----------|---------|--------|
| `HOST` / `PORT` | server bind | Listen address; container uses `0.0.0.0:$PORT`. Defaults `127.0.0.1:8787`. |
| `HOUSECHECK_DB` | startup | Path to the serving SQLite DB. Default `data/housecheck.db`. |
| `CORS_ALLOWED_ORIGIN` | CORS | If set to an origin (in production: `https://housecheck-wine.vercel.app`), CORS is restricted to exactly that origin for `GET`+`POST` with a JSON `content-type`. If unset (or blank/invalid), falls back to **permissive** for local dev. The active mode is logged at startup. |
| `OPENROUTER_API_KEY` | `POST /summary` | Enables the optional LLM summary. Unset (or blank) → `/summary` returns `501`. Read once at startup, not per request. Never commit it; set it as a deploy secret. |
| `OPENROUTER_MODEL` | `POST /summary`, `POST /agent/chat` | Model slug passed to OpenRouter. Defaults to `anthropic/claude-haiku-4.5`. A `:free` model is acceptable for this demo: the grounding facts are entirely **public** NYC building data, and the user's own rent never reaches an LLM (that is `/rent-fairness`). The exposure is whatever a user types. Use a paid zero-data-retention model before collecting any personal data. The server warns at startup when the configured model ends in `:free`. |

---

## Rate limiting (implementation note)

`app_with_state` applies `tower::limit::ConcurrencyLimitLayer(64)`. We first evaluated
`tower_governor` 0.8 for a per-client (~30 req/s) limit — it does support axum 0.8, but its default
`PeerIpKeyExtractor` requires `ConnectInfo<SocketAddr>` (via `into_make_service_with_connect_info`),
which the `axum-test` mock transport used by the test suite does not populate, so it would fail
every test. The concurrency limit bounds resource use on the public API and integrates cleanly with
both the real server and the test transport.
