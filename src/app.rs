use sqlx::PgPool;

use crate::config::Settings;

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub settings: Settings,
}
