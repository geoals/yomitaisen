use std::env;

pub struct Config {
    pub port: u16,
    pub database_url: String,
}

impl Config {
    pub fn from_env() -> Self {
        Self {
            port: env::var("PORT")
                .ok()
                .and_then(|p| p.parse().ok())
                .unwrap_or(3000),
            database_url: env::var("DATABASE_URL")
                .unwrap_or_else(|_| "sqlite:data.db?mode=rwc".to_string()),
        }
    }

    pub fn addr(&self) -> String {
        format!("0.0.0.0:{}", self.port)
    }
}
