use actix_web::{get, web, HttpResponse, Responder};
use serde::Serialize;

use crate::app::AppState;

#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
}

#[get("/health/live")]
async fn live() -> impl Responder {
    HttpResponse::Ok().json(HealthResponse { status: "ok" })
}

#[get("/health/ready")]
async fn ready(state: web::Data<AppState>) -> HttpResponse {
    match sqlx::query("SELECT 1").execute(&state.pool).await {
        Ok(_) => HttpResponse::Ok().json(HealthResponse { status: "ready" }),
        Err(_) => HttpResponse::ServiceUnavailable().json(HealthResponse {
            status: "not_ready",
        }),
    }
}

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(live).service(ready);
}
