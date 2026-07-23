use axum::{
    extract::{Path, Query, State},
    http::{header, HeaderValue, Method, StatusCode},
    response::{IntoResponse, Json},
    routing::get,
    Router,
};
use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};
use tower::limit::ConcurrencyLimitLayer;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

use model::{HealthCard, ScoreBreakdown, Stabilization, ViolationCounts};
use store::{
    get_all_buildings, get_building, get_open_violations, get_snapshot_year, get_tract_median,
};

/// Build the shared async HTTP client used by `/search` (NYC GeoSearch). rustls-only, short
/// timeout, and a UA so the upstream can attribute traffic.
fn http_client() -> reqwest::Client {
    reqwest::Client::builder()
        .user_agent("housecheck-api/0.1")
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .expect("build reqwest client")
}

/// Default scoring year for a DB with no `meta` snapshot row (e.g. the fixture DB).
const DEFAULT_SNAPSHOT_YEAR: i32 = 2026;

/// Shared app state: a single SQLite connection behind a mutex, plus the snapshot year the
/// DB was built for.
/// (Read-mostly reference data + a curated set → a single connection is fine for the MVP.)
#[derive(Clone)]
pub struct AppState {
    conn: Arc<Mutex<rusqlite::Connection>>,
    /// Year used for recency in scoring, read from the DB's `meta` at startup (not the wall
    /// clock) so serving matches the snapshot the ingest recorded. Fixture DBs have no `meta`
    /// row → `DEFAULT_SNAPSHOT_YEAR`.
    snapshot_year: i32,
    /// Async HTTP client for outbound calls (`/search` → NYC GeoSearch). Cloneable + pooled.
    http: reqwest::Client,
}

impl AppState {
    pub fn from_path(path: &str) -> anyhow::Result<Self> {
        let conn = store::open_db(path)?;
        store::migrate(&conn)?;
        let snapshot_year = get_snapshot_year(&conn)?.unwrap_or(DEFAULT_SNAPSHOT_YEAR);
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
            snapshot_year,
            http: http_client(),
        })
    }

    /// In-memory DB seeded with fixtures — used by tests.
    pub fn in_memory_fixture() -> anyhow::Result<Self> {
        let conn = store::open_db(":memory:")?;
        store::migrate(&conn)?;
        store::insert_fixture(&conn)?;
        // The fixture DB writes no `meta` snapshot row, so this falls back to the default.
        let snapshot_year = get_snapshot_year(&conn)?.unwrap_or(DEFAULT_SNAPSHOT_YEAR);
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
            snapshot_year,
            http: http_client(),
        })
    }
}

pub fn app_with_state(state: AppState) -> Router {
    Router::new()
        .route("/health", get(|| async { "ok" }))
        .route("/building/{bbl}", get(building_handler))
        .route("/buildings", get(buildings_handler))
        .route("/compare", get(compare_handler))
        .route("/search", get(search_handler))
        .route("/rent-fairness", axum::routing::post(rent_fairness_handler))
        .route("/summary", axum::routing::post(summary_handler))
        .layer(TraceLayer::new_for_http())
        // Rate limiting: we evaluated `tower_governor` 0.8 (which does support axum 0.8), but its
        // per-client `PeerIpKeyExtractor` needs `ConnectInfo<SocketAddr>` from
        // `into_make_service_with_connect_info` — which the `axum-test` mock transport used by
        // this crate's test suite does not populate, so it would 500 every test. Per the plan's
        // fallback, we use `ConcurrencyLimitLayer(64)` instead: it caps in-flight requests
        // (bounding resource use on the public API) and integrates cleanly with both transports.
        .layer(ConcurrencyLimitLayer::new(64))
        .layer(cors_layer())
        .with_state(state)
}

/// Build the CORS layer from the environment.
///
/// - `CORS_ALLOWED_ORIGIN` set (e.g. the Vercel URL) → allow exactly that origin for GET+POST
///   with a JSON `content-type`. Lets prod tighten to one origin with no code change.
/// - unset (or blank / unparseable) → `CorsLayer::permissive()` for local dev.
///
/// The active mode is logged at startup so the running config is auditable.
fn cors_layer() -> CorsLayer {
    match std::env::var("CORS_ALLOWED_ORIGIN") {
        Ok(origin) if !origin.trim().is_empty() => {
            let origin = origin.trim();
            match origin.parse::<HeaderValue>() {
                Ok(value) => {
                    tracing::info!(origin = %origin, "CORS: restricted to configured origin");
                    CorsLayer::new()
                        .allow_origin(value)
                        .allow_methods([Method::GET, Method::POST])
                        .allow_headers([header::CONTENT_TYPE])
                }
                Err(e) => {
                    tracing::warn!(error = %e, origin = %origin,
                        "CORS_ALLOWED_ORIGIN is not a valid origin; falling back to permissive");
                    CorsLayer::permissive()
                }
            }
        }
        _ => {
            tracing::info!("CORS: permissive (local dev); set CORS_ALLOWED_ORIGIN to restrict");
            CorsLayer::permissive()
        }
    }
}

/// Back-compat helper for the `main` fn / simplest tests.
pub fn app() -> Router {
    let state = AppState::in_memory_fixture().expect("fixture state");
    app_with_state(state)
}

/// Log the real error server-side; return a generic message to the client so a public
/// API never leaks internal detail (table/column names, file paths) from rusqlite errors.
fn internal_error(context: &str, e: impl std::fmt::Display) -> axum::response::Response {
    tracing::error!(error = %e, context, "internal error");
    (StatusCode::INTERNAL_SERVER_ERROR, "internal error").into_response()
}

/// Build a full Health Card for one BBL from the serving DB.
///
/// `Ok(None)` means the BBL isn't in the curated set (→ 404 / skip); `Err` is a real DB failure
/// (→ 500). Shared by `/building`, `/compare`, and `/summary` so all three stay in lockstep.
fn card_for(
    conn: &rusqlite::Connection,
    snapshot_year: i32,
    bbl: &str,
) -> anyhow::Result<Option<HealthCard>> {
    let building = match get_building(conn, bbl)? {
        Some(b) => b,
        None => return Ok(None),
    };
    let violations = get_open_violations(conn, bbl)?;

    let condition = scoring::condition_score(&violations, snapshot_year);
    let legal = scoring::legal_score(&building);
    let neighborhood = scoring::neighborhood_score(building.complaints_311);
    let (accessibility, access_likelihood) = scoring::access_likelihood(&building);
    let total = scoring::total_score(condition, legal, neighborhood, accessibility);

    Ok(Some(HealthCard {
        open_violations: ViolationCounts::open_from(&violations),
        score: ScoreBreakdown {
            total,
            condition,
            legal,
            neighborhood,
            accessibility,
        },
        access_likelihood,
        // Honest three-state signal derived from the stored rent-stabilization data (JustFix
        // nyc-doffer, from NYC DOF Statement-of-Account records, latest year 2024). Carries the
        // unit count for the "likely" wording; the message never overstates a match.
        stabilization: Stabilization::from_units(
            building.rent_stabilized,
            building.rent_stab_units,
        ),
        building,
    }))
}

async fn building_handler(
    State(state): State<AppState>,
    Path(bbl): Path<String>,
) -> impl IntoResponse {
    let snapshot_year = state.snapshot_year;
    // Recover from a poisoned mutex instead of panicking: one prior panic-with-lock-held
    // would otherwise brick every subsequent request on a public server.
    let conn = state.conn.lock().unwrap_or_else(|e| e.into_inner());

    match card_for(&conn, snapshot_year, &bbl) {
        Ok(Some(card)) => (StatusCode::OK, Json(card)).into_response(),
        Ok(None) => (StatusCode::NOT_FOUND, "building not found").into_response(),
        Err(e) => internal_error("database query failed", e),
    }
}

/// Maximum number of buildings a single `/compare` request will score, to bound work.
const COMPARE_MAX_BBLS: usize = 4;

#[derive(Deserialize)]
struct CompareParams {
    bbls: String,
}

#[derive(Serialize)]
struct CompareResponse {
    buildings: Vec<HealthCard>,
}

/// `GET /compare?bbls=a,b,c` — side-by-side Health Cards for up to `COMPARE_MAX_BBLS` buildings.
/// Each card is built with the exact same logic as `/building`. BBLs not in the curated set are
/// silently skipped (so a mixed list still returns the ones we have). `400` if `bbls` is
/// missing/empty.
async fn compare_handler(
    State(state): State<AppState>,
    Query(params): Query<CompareParams>,
) -> impl IntoResponse {
    // Split, trim, drop blanks, dedupe-preserving-order, then cap the count.
    let mut seen = std::collections::HashSet::new();
    let bbls: Vec<&str> = params
        .bbls
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .filter(|s| seen.insert(*s))
        .take(COMPARE_MAX_BBLS)
        .collect();
    if bbls.is_empty() {
        return (StatusCode::BAD_REQUEST, "bbls query param required").into_response();
    }

    let snapshot_year = state.snapshot_year;
    let conn = state.conn.lock().unwrap_or_else(|e| e.into_inner());
    let mut buildings = Vec::with_capacity(bbls.len());
    for bbl in bbls {
        match card_for(&conn, snapshot_year, bbl) {
            Ok(Some(card)) => buildings.push(card),
            Ok(None) => {} // not in the curated set → silently skip
            Err(e) => return internal_error("database query failed", e),
        }
    }
    (StatusCode::OK, Json(CompareResponse { buildings })).into_response()
}

/// `GET /buildings` — compact list/map feed for the frontend. Total score is computed on the
/// fly per row (~250 rows is trivial), so the list stays in lockstep with `/building/{bbl}`.
async fn buildings_handler(State(state): State<AppState>) -> impl IntoResponse {
    let snapshot_year = state.snapshot_year;
    let conn = state.conn.lock().unwrap_or_else(|e| e.into_inner());
    let buildings = match get_all_buildings(&conn) {
        Ok(b) => b,
        Err(e) => return internal_error("database query failed", e),
    };
    let mut out = Vec::with_capacity(buildings.len());
    for b in &buildings {
        let violations = match get_open_violations(&conn, &b.bbl) {
            Ok(v) => v,
            Err(e) => return internal_error("database query failed", e),
        };
        let condition = scoring::condition_score(&violations, snapshot_year);
        let legal = scoring::legal_score(b);
        let neighborhood = scoring::neighborhood_score(b.complaints_311);
        let (accessibility, _) = scoring::access_likelihood(b);
        let total = scoring::total_score(condition, legal, neighborhood, accessibility);
        out.push(model::BuildingListItem {
            bbl: b.bbl.clone(),
            address: b.address.clone(),
            latitude: b.latitude,
            longitude: b.longitude,
            score: total,
        });
    }
    (StatusCode::OK, Json(out)).into_response()
}

#[derive(Deserialize)]
struct RentFairnessReq {
    bbl: String,
    monthly_rent: i32,
}

async fn rent_fairness_handler(
    State(state): State<AppState>,
    Json(req): Json<RentFairnessReq>,
) -> impl IntoResponse {
    if req.monthly_rent <= 0 {
        return (StatusCode::BAD_REQUEST, "monthly_rent must be positive").into_response();
    }
    // Recover from a poisoned mutex instead of panicking: one prior panic-with-lock-held
    // would otherwise brick every subsequent request on a public server.
    let conn = state.conn.lock().unwrap_or_else(|e| e.into_inner());
    let building = match get_building(&conn, &req.bbl) {
        Ok(Some(b)) => b,
        Ok(None) => return (StatusCode::NOT_FOUND, "building not found").into_response(),
        Err(e) => return internal_error("database query failed", e),
    };
    let median = match get_tract_median(&conn, &building.tract_geoid) {
        Ok(Some(m)) => m,
        Ok(None) => return (StatusCode::NOT_FOUND, "no rent data for tract").into_response(),
        Err(e) => return internal_error("database query failed", e),
    };
    let (pct, verdict) = scoring::rent_fairness(req.monthly_rent, median);
    let body = model::RentFairness {
        bbl: req.bbl,
        user_rent: req.monthly_rent,
        tract_median: median,
        pct_vs_median: pct,
        verdict,
        // Second comparator: embedded HUD FMRs by bedroom for the NYC metro area, so the
        // frontend can show "vs HUD FMR" next to the Census tract median.
        hud_fmr: model::HudFmr::ny_metro_fy2026(),
    };
    (StatusCode::OK, Json(body)).into_response()
}

#[derive(Deserialize)]
struct SearchParams {
    address: String,
}

#[derive(Serialize)]
struct SearchResult {
    bbl: String,
    label: String,
    in_curated_set: bool,
}

/// Pull a BBL out of a GeoSearch feature's `properties`. GeoSearch exposes it either at
/// `addendum.pad.bbl` (full result) or `pad_bbl` (compact) — and as a string or a number — so
/// both shapes are handled. Returns the canonical 10-digit BBL string.
fn geosearch_bbl(props: &serde_json::Value) -> Option<String> {
    let raw = props
        .get("addendum")
        .and_then(|a| a.get("pad"))
        .and_then(|p| p.get("bbl"))
        .or_else(|| props.get("pad_bbl"))?;
    match raw {
        serde_json::Value::String(s) => {
            let t = s.trim();
            (!t.is_empty()).then(|| t.to_string())
        }
        serde_json::Value::Number(n) => n.as_u64().map(|v| v.to_string()),
        _ => None,
    }
}

/// `GET /search?address=<text>` — live-geocode free-text via NYC GeoSearch, return the top
/// match's BBL, label, and whether it's in our curated DB. 404 when GeoSearch finds nothing;
/// 502 when the upstream call/parse fails.
async fn search_handler(
    State(state): State<AppState>,
    Query(params): Query<SearchParams>,
) -> impl IntoResponse {
    let text = params.address.trim();
    if text.is_empty() {
        return (StatusCode::BAD_REQUEST, "address query param required").into_response();
    }

    let resp = match state
        .http
        .get("https://geosearch.planninglabs.nyc/v2/search")
        .query(&[("text", text), ("size", "1")])
        .send()
        .await
        .and_then(|r| r.error_for_status())
    {
        Ok(r) => r,
        Err(e) => {
            tracing::error!(error = %e, "geosearch upstream failed");
            return (StatusCode::BAD_GATEWAY, "geocoding upstream failed").into_response();
        }
    };
    let json: serde_json::Value = match resp.json().await {
        Ok(j) => j,
        Err(e) => {
            tracing::error!(error = %e, "geosearch decode failed");
            return (StatusCode::BAD_GATEWAY, "geocoding upstream failed").into_response();
        }
    };

    let feature = json
        .get("features")
        .and_then(|f| f.as_array())
        .and_then(|a| a.first());
    let Some(props) = feature.and_then(|f| f.get("properties")) else {
        return (StatusCode::NOT_FOUND, "no match for address").into_response();
    };
    let Some(bbl) = geosearch_bbl(props) else {
        return (StatusCode::NOT_FOUND, "no BBL for address").into_response();
    };
    let label = props
        .get("label")
        .and_then(|l| l.as_str())
        .unwrap_or("")
        .to_string();

    // Membership check against our DB. Locked AFTER the awaits — the guard never crosses one.
    let in_curated_set = {
        let conn = state.conn.lock().unwrap_or_else(|e| e.into_inner());
        match get_building(&conn, &bbl) {
            Ok(b) => b.is_some(),
            Err(e) => return internal_error("database query failed", e),
        }
    };

    (
        StatusCode::OK,
        Json(SearchResult {
            bbl,
            label,
            in_curated_set,
        }),
    )
        .into_response()
}

/// System prompt for `/summary`. Honest and hedged — it must not invent facts.
const SUMMARY_SYSTEM_PROMPT: &str = "You are a plain-spoken NYC renter's advocate. In 2-3 \
sentences, explain what this building's data means for a prospective renter. Be concrete and \
honest; do not invent facts.";

/// OpenRouter's OpenAI-compatible chat-completions endpoint.
const OPENROUTER_URL: &str = "https://openrouter.ai/api/v1/chat/completions";
/// The team's free model on OpenRouter.
const SUMMARY_MODEL: &str = "nvidia/nemotron-3-ultra-550b-a55b:free";

#[derive(Deserialize)]
struct SummaryReq {
    bbl: String,
}

#[derive(Serialize)]
struct SummaryResp {
    bbl: String,
    summary: String,
}

/// `POST /summary` — optional plain-language summary of a building's Health Card via OpenRouter.
///
/// - `404` if the BBL isn't in the curated set.
/// - `501 Not Implemented` (with a JSON error) if `OPENROUTER_API_KEY` is unset — this endpoint
///   is optional, so a missing key disables it rather than erroring the server.
/// - `502 Bad Gateway` if the upstream call/parse fails.
async fn summary_handler(
    State(state): State<AppState>,
    Json(req): Json<SummaryReq>,
) -> impl IntoResponse {
    let snapshot_year = state.snapshot_year;

    // Build the card (and grab the tract median for rent context) under the lock, then drop it
    // before any await — the guard never crosses the network call.
    let (card, tract_median) = {
        let conn = state.conn.lock().unwrap_or_else(|e| e.into_inner());
        match card_for(&conn, snapshot_year, &req.bbl) {
            Ok(Some(card)) => {
                let median = get_tract_median(&conn, &card.building.tract_geoid)
                    .ok()
                    .flatten();
                (card, median)
            }
            Ok(None) => return (StatusCode::NOT_FOUND, "building not found").into_response(),
            Err(e) => return internal_error("database query failed", e),
        }
    };

    // Optional feature: no key → advertise it as disabled, don't error the server.
    let api_key = match std::env::var("OPENROUTER_API_KEY") {
        Ok(k) if !k.trim().is_empty() => k,
        _ => {
            return (
                StatusCode::NOT_IMPLEMENTED,
                Json(serde_json::json!({
                    "error": "summary disabled — set OPENROUTER_API_KEY"
                })),
            )
                .into_response();
        }
    };

    // The card's key facts. `/summary` takes only a BBL (no user rent), so "rent-fairness" is
    // surfaced as the neighborhood tract median for context rather than a personalized percentage.
    let rent_context = match tract_median {
        Some(m) => format!("neighborhood median gross rent ${m}/mo (Census tract)"),
        None => "no reliable neighborhood median rent available".to_string(),
    };
    let v = &card.open_violations;
    let user_facts = format!(
        "Building: {address} (BBL {bbl}), built {year_built}, {units_res} residential units.\n\
         Overall health score: {total}/100 (condition {condition}, legal protection {legal}, \
         neighborhood {neighborhood}, accessibility {accessibility}).\n\
         Open HPD violations: {c} class-C (most serious), {b} class-B, {a} class-A.\n\
         Rent-stabilization signal: {stab_message} ({stab_status}).\n\
         Rent context: {rent_context}.\n\
         Accessibility likelihood: {access}.\n\
         Nearby 311 complaints: {complaints_311}.",
        address = card.building.address,
        bbl = card.building.bbl,
        year_built = card.building.year_built,
        units_res = card.building.units_res,
        total = card.score.total,
        condition = card.score.condition,
        legal = card.score.legal,
        neighborhood = card.score.neighborhood,
        accessibility = card.score.accessibility,
        c = v.c,
        b = v.b,
        a = v.a,
        stab_message = card.stabilization.message,
        stab_status = card.stabilization.status,
        rent_context = rent_context,
        access = card.access_likelihood,
        complaints_311 = card.building.complaints_311,
    );

    let payload = serde_json::json!({
        "model": SUMMARY_MODEL,
        "messages": [
            { "role": "system", "content": SUMMARY_SYSTEM_PROMPT },
            { "role": "user", "content": user_facts },
        ],
    });

    let resp = match state
        .http
        .post(OPENROUTER_URL)
        .bearer_auth(&api_key)
        // Per-request override of the shared client's 10s default — LLMs can be slower.
        .timeout(std::time::Duration::from_secs(20))
        .json(&payload)
        .send()
        .await
        .and_then(|r| r.error_for_status())
    {
        Ok(r) => r,
        Err(e) => {
            tracing::error!(error = %e, "openrouter upstream failed");
            return (StatusCode::BAD_GATEWAY, "summary upstream failed").into_response();
        }
    };

    let json: serde_json::Value = match resp.json().await {
        Ok(j) => j,
        Err(e) => {
            tracing::error!(error = %e, "openrouter decode failed");
            return (StatusCode::BAD_GATEWAY, "summary upstream failed").into_response();
        }
    };

    // OpenAI-compatible shape: choices[0].message.content.
    let summary = json
        .get("choices")
        .and_then(|c| c.as_array())
        .and_then(|a| a.first())
        .and_then(|m| m.get("message"))
        .and_then(|m| m.get("content"))
        .and_then(|c| c.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty());

    match summary {
        Some(s) => (
            StatusCode::OK,
            Json(SummaryResp {
                bbl: req.bbl,
                summary: s.to_string(),
            }),
        )
            .into_response(),
        None => {
            tracing::error!("openrouter response had no summary content");
            (StatusCode::BAD_GATEWAY, "summary upstream failed").into_response()
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    let db = std::env::var("HOUSECHECK_DB").unwrap_or_else(|_| "data/housecheck.db".to_string());
    let state = AppState::from_path(&db)?;
    // Bind host/port from env so a container can listen on 0.0.0.0:$PORT (Fly/Shuttle);
    // defaults keep local dev on 127.0.0.1:8787.
    let host = std::env::var("HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let port = std::env::var("PORT").unwrap_or_else(|_| "8787".to_string());
    let listener = tokio::net::TcpListener::bind(format!("{host}:{port}")).await?;
    tracing::info!("listening on {}", listener.local_addr()?);
    axum::serve(listener, app_with_state(state)).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum_test::TestServer;
    use model::HealthCard;

    fn test_server() -> TestServer {
        // Shared in-memory DB seeded with fixtures, wrapped in the app state.
        let state = AppState::in_memory_fixture().unwrap();
        TestServer::new(app_with_state(state)).unwrap()
    }

    #[tokio::test]
    async fn health_returns_ok() {
        let server = test_server();
        let res = server.get("/health").await;
        res.assert_status_ok();
        res.assert_text("ok");
    }

    #[tokio::test]
    async fn building_returns_scored_card() {
        let server = test_server();
        let res = server.get("/building/3000020002").await;
        res.assert_status_ok();
        let card: HealthCard = res.json();
        assert_eq!(card.building.bbl, "3000020002");
        assert!(card.score.total <= 100);
        // walk-up with open C+B violations -> some open violations present
        assert!(card.open_violations.c >= 1);
        assert_eq!(card.access_likelihood, "Lower"); // 1930 walk-up, 4 floors, pre-FHA
    }

    #[tokio::test]
    async fn fixture_snapshot_year_defaults_and_scores() {
        // The fixture DB has no `meta` snapshot row, so the server must fall back to 2026 and
        // still score a card (regression guard for the removed hardcoded SCORING_YEAR const).
        let state = AppState::in_memory_fixture().unwrap();
        assert_eq!(state.snapshot_year, DEFAULT_SNAPSHOT_YEAR);
        let server = TestServer::new(app_with_state(state)).unwrap();
        let res = server.get("/building/3000020002").await;
        res.assert_status_ok();
        let card: HealthCard = res.json();
        // 3000020002 has an open C (2026) + open B (2025); at snapshot 2026 both are "recent"
        // (<=2 yrs) → penalty 15*2 + 7*2 = 44 → condition 56. A wrong year would shift this.
        assert_eq!(card.score.condition, 56);
    }

    #[tokio::test]
    async fn unknown_building_is_404() {
        let server = test_server();
        let res = server.get("/building/9999999999").await;
        res.assert_status_not_found();
    }

    use model::RentFairness;
    use serde_json::json;

    #[tokio::test]
    async fn rent_fairness_returns_pct_vs_median() {
        let server = test_server();
        let res = server
            .post("/rent-fairness")
            .json(&json!({"bbl": "3000010001", "monthly_rent": 3000}))
            .await;
        res.assert_status_ok();
        let rf: RentFairness = res.json();
        assert_eq!(rf.tract_median, 2500);
        assert_eq!(rf.pct_vs_median.round() as i32, 20);
        assert!(rf.verdict.contains("above"));
    }

    #[tokio::test]
    async fn rent_fairness_rejects_nonpositive_rent() {
        let server = test_server();
        let res = server
            .post("/rent-fairness")
            .json(&json!({"bbl": "3000010001", "monthly_rent": 0}))
            .await;
        res.assert_status_bad_request();
    }

    #[tokio::test]
    async fn rent_fairness_unknown_bbl_is_404() {
        let server = test_server();
        let res = server
            .post("/rent-fairness")
            .json(&json!({"bbl": "9999999999", "monthly_rent": 3000}))
            .await;
        res.assert_status_not_found();
    }

    #[tokio::test]
    async fn rent_fairness_includes_hud_fmr() {
        let server = test_server();
        let res = server
            .post("/rent-fairness")
            .json(&json!({"bbl": "3000010001", "monthly_rent": 3000}))
            .await;
        res.assert_status_ok();
        let rf: RentFairness = res.json();
        // The embedded FY2026 NYC-metro HUD FMRs travel alongside the tract-median comparison.
        assert_eq!(rf.hud_fmr.fiscal_year, 2026);
        assert_eq!(rf.hud_fmr.two_br, 2910);
        assert!(rf.hud_fmr.area.contains("HUD Metro FMR Area"));
    }

    #[tokio::test]
    async fn building_card_includes_stabilization_signal() {
        let server = test_server();
        // Fixture building 1 has rent_stabilized = 1 with 12 units → "likely" wording that
        // surfaces the unit count, and the count travels in the building payload.
        let res = server.get("/building/3000010001").await;
        res.assert_status_ok();
        let card: HealthCard = res.json();
        assert_eq!(card.stabilization.status, "likely");
        assert!(card.stabilization.message.contains("12 units"));
        assert_eq!(card.building.rent_stab_units, Some(12));
        // Building 2 has rent_stabilized = NULL → "unverified" (never overstated).
        let res2 = server.get("/building/3000020002").await;
        let card2: HealthCard = res2.json();
        assert_eq!(card2.stabilization.status, "unverified");
        assert_eq!(card2.building.rent_stab_units, None);
    }

    #[tokio::test]
    async fn buildings_list_returns_scored_items() {
        let server = test_server();
        let res = server.get("/buildings").await;
        res.assert_status_ok();
        let items: Vec<model::BuildingListItem> = res.json();
        assert_eq!(items.len(), 2);
        // Ordered by BBL; carries stored coordinates + a computed total score.
        assert_eq!(items[0].bbl, "3000010001");
        assert!(items[0].latitude.is_some());
        assert!(items[0].score <= 100);
    }

    #[tokio::test]
    async fn search_rejects_blank_address() {
        let server = test_server();
        // Whitespace-only address trims to empty → 400 before any upstream call.
        let res = server.get("/search?address=%20%20").await;
        res.assert_status_bad_request();
    }

    #[tokio::test]
    async fn compare_returns_multiple_cards() {
        let server = test_server();
        let res = server.get("/compare?bbls=3000010001,3000020002").await;
        res.assert_status_ok();
        let body: serde_json::Value = res.json();
        let buildings = body["buildings"].as_array().expect("buildings array");
        // Both fixture BBLs resolve → two full Health Cards, in request order.
        assert_eq!(buildings.len(), 2);
        assert_eq!(buildings[0]["building"]["bbl"], "3000010001");
        assert_eq!(buildings[1]["building"]["bbl"], "3000020002");
        // Cards carry the same shape as /building (scored breakdown present).
        assert!(buildings[0]["score"]["total"].is_number());
    }

    #[tokio::test]
    async fn compare_skips_unknown_bbls() {
        let server = test_server();
        // An unknown BBL sandwiched between two real ones is silently dropped.
        let res = server
            .get("/compare?bbls=3000010001,9999999999,3000020002")
            .await;
        res.assert_status_ok();
        let body: serde_json::Value = res.json();
        assert_eq!(body["buildings"].as_array().unwrap().len(), 2);
    }

    #[tokio::test]
    async fn compare_requires_bbls() {
        let server = test_server();
        // Missing param entirely → Query rejection → 400.
        server.get("/compare").await.assert_status_bad_request();
        // Present but whitespace-only → our explicit empty guard → 400.
        server
            .get("/compare?bbls=%20")
            .await
            .assert_status_bad_request();
    }

    #[tokio::test]
    async fn summary_returns_501_when_key_unset() {
        // Disable the optional LLM path so no network call is attempted in tests.
        std::env::remove_var("OPENROUTER_API_KEY");
        let server = test_server();
        let res = server
            .post("/summary")
            .json(&json!({"bbl": "3000010001"}))
            .await;
        res.assert_status(StatusCode::NOT_IMPLEMENTED);
        let body: serde_json::Value = res.json();
        assert_eq!(body["error"], "summary disabled — set OPENROUTER_API_KEY");
    }

    #[tokio::test]
    async fn summary_unknown_bbl_is_404() {
        // 404 is returned before the key check, so this never touches the network.
        let server = test_server();
        let res = server
            .post("/summary")
            .json(&json!({"bbl": "9999999999"}))
            .await;
        res.assert_status_not_found();
    }

    #[test]
    fn geosearch_bbl_handles_both_shapes_and_types() {
        // Full result: properties.addendum.pad.bbl as a string.
        let full = json!({"addendum": {"pad": {"bbl": "3018420001"}}});
        assert_eq!(geosearch_bbl(&full).as_deref(), Some("3018420001"));
        // Compact result: properties.pad_bbl fallback.
        let compact = json!({"pad_bbl": "3000010001"});
        assert_eq!(geosearch_bbl(&compact).as_deref(), Some("3000010001"));
        // BBL shipped as a JSON number.
        let numeric = json!({"addendum": {"pad": {"bbl": 3018420001u64}}});
        assert_eq!(geosearch_bbl(&numeric).as_deref(), Some("3018420001"));
        // No BBL anywhere → None.
        assert!(geosearch_bbl(&json!({"label": "somewhere"})).is_none());
    }
}
