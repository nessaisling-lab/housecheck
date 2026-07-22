use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Json},
    routing::get,
    Router,
};
use std::sync::{Arc, Mutex};

use serde::Deserialize;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

use model::{HealthCard, ScoreBreakdown, ViolationCounts};
use store::{get_building, get_open_violations, get_tract_median};

/// Shared app state: a single SQLite connection behind a mutex.
/// (Read-mostly reference data + a curated set → a single connection is fine for the MVP.)
#[derive(Clone)]
pub struct AppState {
    conn: Arc<Mutex<rusqlite::Connection>>,
}

impl AppState {
    pub fn from_path(path: &str) -> anyhow::Result<Self> {
        let conn = store::open_db(path)?;
        store::migrate(&conn)?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    /// In-memory DB seeded with fixtures — used by tests.
    pub fn in_memory_fixture() -> anyhow::Result<Self> {
        let conn = store::open_db(":memory:")?;
        store::migrate(&conn)?;
        store::insert_fixture(&conn)?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }
}

/// Year used for recency in scoring. Centralized so it's the single place to bump.
const SCORING_YEAR: i32 = 2026;

pub fn app_with_state(state: AppState) -> Router {
    Router::new()
        .route("/health", get(|| async { "ok" }))
        .route("/building/{bbl}", get(building_handler))
        .route("/rent-fairness", axum::routing::post(rent_fairness_handler))
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive()) // MVP: tighten to the Vercel origin before launch
        .with_state(state)
}

/// Back-compat helper for the `main` fn / simplest tests.
pub fn app() -> Router {
    let state = AppState::in_memory_fixture().expect("fixture state");
    app_with_state(state)
}

async fn building_handler(
    State(state): State<AppState>,
    Path(bbl): Path<String>,
) -> impl IntoResponse {
    let conn = state.conn.lock().unwrap();

    let building = match get_building(&conn, &bbl) {
        Ok(Some(b)) => b,
        Ok(None) => return (StatusCode::NOT_FOUND, "building not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    };
    let violations = match get_open_violations(&conn, &bbl) {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    };

    let condition = scoring::condition_score(&violations, SCORING_YEAR);
    let legal = scoring::legal_score(&building);
    let neighborhood = scoring::neighborhood_score(building.complaints_311);
    let (accessibility, access_likelihood) = scoring::access_likelihood(&building);
    let total = scoring::total_score(condition, legal, neighborhood, accessibility);

    let card = HealthCard {
        open_violations: ViolationCounts::open_from(&violations),
        score: ScoreBreakdown {
            total,
            condition,
            legal,
            neighborhood,
            accessibility,
        },
        access_likelihood,
        building,
    };
    (StatusCode::OK, Json(card)).into_response()
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
    let conn = state.conn.lock().unwrap();
    let building = match get_building(&conn, &req.bbl) {
        Ok(Some(b)) => b,
        Ok(None) => return (StatusCode::NOT_FOUND, "building not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    };
    let median = match get_tract_median(&conn, &building.tract_geoid) {
        Ok(Some(m)) => m,
        Ok(None) => return (StatusCode::NOT_FOUND, "no rent data for tract").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    };
    let (pct, verdict) = scoring::rent_fairness(req.monthly_rent, median);
    let body = model::RentFairness {
        bbl: req.bbl,
        user_rent: req.monthly_rent,
        tract_median: median,
        pct_vs_median: pct,
        verdict,
    };
    (StatusCode::OK, Json(body)).into_response()
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    let db = std::env::var("HOUSECHECK_DB").unwrap_or_else(|_| "data/housecheck.db".to_string());
    let state = AppState::from_path(&db)?;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:8787").await?;
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
}
