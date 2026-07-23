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
    "message": "Unverified"
  }
}
```

Field notes:

- `building.latitude` / `building.longitude` — building centroid from PLUTO (`null` if it was
  never geocoded). Same coordinates the `/buildings` map feed uses.
- `building.restaurant_grade` — letter grade (`"A"`/`"B"`/`"C"`) of the nearest DOHMH-graded
  restaurant within ~200 m, or `null`. **Neighborhood context only — never part of any score.**
- `access_likelihood` — one of `"Higher"`, `"Mixed"`, `"Lower"`. A likelihood, not a certification.
- `stabilization` — three honest states derived from `building.rent_stabilized`:
  | `rent_stabilized` | `status`      | `message` |
  |-------------------|---------------|-----------|
  | `true`            | `on_record`   | `Likely rent-stabilized — a signal, not a legal ruling` |
  | `false`           | `not_found`   | `No record found — public lists are incomplete` |
  | `null`            | `unverified`  | `Unverified` |

  > No live rent-stabilization dataset is wired into ingest yet, so real rows currently read
  > `unverified`. The wording is intentionally hedged and never overstates a match.

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
    { "building": { "bbl": "3018420001", "...": "..." }, "score": { "total": 72, "...": "..." }, "open_violations": { "a": 0, "b": 1, "c": 1 }, "access_likelihood": "Lower", "stabilization": { "status": "unverified", "message": "Unverified" } },
    { "building": { "bbl": "3018420015", "...": "..." }, "score": { "total": 88, "...": "..." }, "open_violations": { "a": 0, "b": 0, "c": 0 }, "access_likelihood": "Higher", "stabilization": { "status": "on_record", "message": "Likely rent-stabilized — a signal, not a legal ruling" } }
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

Live-geocode free-text via NYC GeoSearch and return the top match's BBL, so the frontend can jump
straight to a building and tell whether it's in the curated set.

- **Query param:** `address` — free-text address (required, non-blank).
- **Success:** `200 OK` with the object below.
- **Errors:** `400 Bad Request` if `address` is missing/blank; `404 Not Found` if GeoSearch has no
  match (or the match has no BBL); `502 Bad Gateway` if the GeoSearch upstream fails or is
  unparseable.

```bash
curl -s 'http://127.0.0.1:8787/search?address=123%20Macon%20Street%20Brooklyn'
```
```json
{
  "bbl": "3018420001",
  "label": "123 Macon Street, Brooklyn, NY, USA",
  "in_curated_set": true
}
```

Field notes:

- `bbl` — canonical 10-digit BBL from the GeoSearch feature (handles both
  `properties.addendum.pad.bbl` and `properties.pad_bbl`, string or number).
- `label` — GeoSearch's human-readable label for the match.
- `in_curated_set` — `true` if that BBL exists in our DB (so `/building/{bbl}` will resolve).

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

## Environment variables

| Variable | Used by | Effect |
|----------|---------|--------|
| `HOST` / `PORT` | server bind | Listen address; container uses `0.0.0.0:$PORT`. Defaults `127.0.0.1:8787`. |
| `HOUSECHECK_DB` | startup | Path to the serving SQLite DB. Default `data/housecheck.db`. |
| `CORS_ALLOWED_ORIGIN` | CORS | If set to an origin (e.g. `https://housecheck.vercel.app`), CORS is restricted to exactly that origin for `GET`+`POST` with a JSON `content-type`. If unset (or blank/invalid), falls back to **permissive** for local dev. The active mode is logged at startup. |
| `OPENROUTER_API_KEY` | `POST /summary` | Enables the optional LLM summary. Unset → `/summary` returns `501`. Never commit it; set it as a deploy secret. |

---

## Rate limiting (implementation note)

`app_with_state` applies `tower::limit::ConcurrencyLimitLayer(64)`. We first evaluated
`tower_governor` 0.8 for a per-client (~30 req/s) limit — it does support axum 0.8, but its default
`PeerIpKeyExtractor` requires `ConnectInfo<SocketAddr>` (via `into_make_service_with_connect_info`),
which the `axum-test` mock transport used by the test suite does not populate, so it would fail
every test. The concurrency limit bounds resource use on the public API and integrates cleanly with
both the real server and the test transport.
