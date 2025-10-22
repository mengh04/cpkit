use crate::models::{CompetitiveCompanionData, Problem};
use crate::storage::ProblemStore;
use anyhow::Result;
use axum::{extract::State, http::StatusCode, response::IntoResponse, routing::post, Json, Router};
use std::sync::Arc;
use tokio::sync::Mutex;
use tower_http::cors::{Any, CorsLayer};

const DEFAULT_PORT: u16 = 10043;

pub type SharedProblemStore = Arc<Mutex<ProblemStore>>;

/// 启动 Competitive Companion 监听服务器
pub async fn start_server(store: SharedProblemStore) -> Result<()> {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .route("/", post(receive_problem))
        .layer(cors)
        .with_state(store);

    let addr = format!("127.0.0.1:{}", DEFAULT_PORT);
    tracing::info!("Competitive Companion server started on {}", addr);

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

/// 接收从 Competitive Companion 发送的问题数据
async fn receive_problem(
    State(store): State<SharedProblemStore>,
    Json(data): Json<CompetitiveCompanionData>,
) -> impl IntoResponse {
    tracing::info!("Received new problem: {}", data.name);

    let problem: Problem = data.into();
    let problem_name = problem.name.clone();

    match store.lock().await.add_problem(problem) {
        Ok(_) => {
            tracing::info!("Problem '{}' saved", problem_name);
            (StatusCode::OK, "Problem received")
        }
        Err(e) => {
            tracing::error!("Failed to save problem: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, "Save failed")
        }
    }
}
