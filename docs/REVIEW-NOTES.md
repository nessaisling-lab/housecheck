# HouseCheck — Backend Code-Review Notes (2026-07-22)

Independent review of the backend-core implementation (commits `ee3a5a3..78d93c3`). Architecture verdict: **sound — safe to build the frontend and real-data ingest on top.** Pure-`scoring` / SQL-owning-`store` / composing-`api` boundaries hold; SQL is parameterized (no injection); scoring is deterministic and clamped.

## Fixed in this branch (post-review hardening)

| Ref | Severity | Issue | Fix |
|---|---|---|---|
| C1 | 🔴 Critical | `rent_fairness` divided by tract median with no guard; real Census B25064 ships suppressed tracts as `0`/`-666666666` → fabricated "% vs median" or `null` | `store::get_tract_median` filters `median_gross_rent > 0`; `scoring::rent_fairness` returns a safe "no reliable median" result for non-positive input. 2 new tests. |
| I1 | 🟠 Important | `.lock().unwrap()` would brick the whole server on a poisoned mutex | Recover with `.lock().unwrap_or_else(\|e\| e.into_inner())` |
| M1 | Minor | Raw `rusqlite` error strings returned to clients + never logged | `internal_error()` helper: `tracing::error!` server-side, generic body to client |
| M2 | Minor | Host/port hardcoded `127.0.0.1:8787` — container can't accept external traffic | Bind `HOST`/`PORT` from env (defaults unchanged for local dev) |

## Deferred to the D8 hardening milestone (tracked, intentional)

| Ref | Issue | Plan |
|---|---|---|
| I2 | Rate-limiting (spec §4b/§7) was dropped from the plan | Add `tower_governor` or `ConcurrencyLimitLayer` before launch; add to roadmap D8 |
| I4 | Single `Arc<Mutex<Connection>>` serializes reads; lock held across CPU-bound scoring | Acceptable for curated MVP. Scale path: `r2d2_sqlite`/`deadpool-sqlite` + `PRAGMA journal_mode=WAL`; pull the `Building`/`Vec<Violation>` out of the lock before scoring |
| M3 | Dead `app()` helper builds a throwaway fixture DB | Remove or use in the `/health` test |
| M4 | Unknown violation classes silently score 0 | At ingest (Plan 2): count/log unrecognized HPD classes instead of dropping |
| M5 | `SELECT *` + positional fixture INSERTs couple to column order | Enumerate columns explicitly |
| M6 | `SCORING_YEAR = 2026` frozen; recency window drifts as calendar advances | Derive from the ingest snapshot date in Plan 2 |
| M7 | Score weights live only in code; spec §4d wants "show the math" surfaced | Return weights / per-axis contributions in the `/building` payload for the frontend |

## Reconciled (not a defect)

- **I3 — `near_ada_subway_m` stored but not in the score.** The refined PRD treats nearby ADA transit as a **neighborhood "context chip"**, shown separately, *not* an input to the access-likelihood badge (which is elevator + build-era). Current behavior matches the PRD. Plan 2 will populate the field from MTA `39hk-dx4f` and the frontend renders it as a chip.
- **CORS `permissive()`** — low risk for this MVP: no cookies, no credentials, all data public. Comment already flags tightening to the Vercel origin before adding any auth.

## Recommended next-session order
C1 ✅ done → I1 ✅ done → I2 (rate-limit) → M1/M2 ✅ done for deploy → then Plan 2 (real ingest) and Plan 3 (frontend).
