use actix_web::{middleware::Logger, web, App, HttpServer};
use atlas_auth::{
    app::AppState, config::Settings, db, oauth::client_repository::OAuthClientRepository, routes,
};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let settings = Settings::from_env()?;
    let socket_addr = settings.socket_addr()?;
    let pool = db::connect(&settings.database_url).await?;
    let client_repository = OAuthClientRepository::postgres(pool.clone());
    let state = AppState { pool, settings };

    tracing::info!(%socket_addr, "starting atlas-auth");

    HttpServer::new(move || {
        App::new()
            .wrap(Logger::default())
            .app_data(web::Data::new(state.clone()))
            .app_data(web::Data::new(client_repository.clone()))
            .configure(routes::health::configure)
            .configure(routes::auth::configure)
            .configure(routes::oauth::configure)
            .configure(routes::oidc::configure)
    })
    .bind(socket_addr)?
    .run()
    .await?;

    Ok(())
}
