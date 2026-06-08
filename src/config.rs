use std::{env, net::SocketAddr};

#[derive(Debug, Clone)]
pub struct Settings {
    pub app_env: String,
    pub server_host: String,
    pub server_port: u16,
    pub database_url: String,
    pub jwt_issuer: String,
    pub jwt_audience: String,
    pub jwt_access_token_ttl_seconds: i64,
    pub jwt_refresh_token_ttl_seconds: i64,
    pub jwt_signing_key_id: String,
    pub jwt_private_key_pem: String,
    pub jwt_public_key_pem: String,
    pub password_pepper: String,
}

impl Settings {
    pub fn from_env() -> anyhow::Result<Self> {
        let _ = dotenvy::dotenv();

        Ok(Self {
            app_env: env_or("APP_ENV", "local"),
            server_host: env_or("SERVER_HOST", "127.0.0.1"),
            server_port: env_or("SERVER_PORT", "8080").parse()?,
            database_url: env::var("DATABASE_URL")?,
            jwt_issuer: env_or("JWT_ISSUER", "atlas-auth"),
            jwt_audience: env_or("JWT_AUDIENCE", "atlas-services"),
            jwt_access_token_ttl_seconds: env_or("JWT_ACCESS_TOKEN_TTL_SECONDS", "900").parse()?,
            jwt_refresh_token_ttl_seconds: env_or("JWT_REFRESH_TOKEN_TTL_SECONDS", "2592000")
                .parse()?,
            jwt_signing_key_id: env_or("JWT_SIGNING_KEY_ID", "local-dev-key"),
            jwt_private_key_pem: env_pem("JWT_PRIVATE_KEY_PEM")?,
            jwt_public_key_pem: env_pem("JWT_PUBLIC_KEY_PEM")?,
            password_pepper: env::var("PASSWORD_PEPPER")?,
        })
    }

    pub fn socket_addr(&self) -> anyhow::Result<SocketAddr> {
        Ok(format!("{}:{}", self.server_host, self.server_port).parse()?)
    }
}

fn env_or(key: &str, default: &str) -> String {
    env::var(key).unwrap_or_else(|_| default.to_string())
}

fn env_pem(key: &str) -> anyhow::Result<String> {
    Ok(env::var(key)?.replace("\\n", "\n"))
}
