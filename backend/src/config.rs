use std::env;

pub struct Config {
    pub port: u16,
}

impl Config {
    pub fn from_env() -> Self {
        dotenvy::dotenv().ok();

        Self {
            port: env::var("PORT")
                .ok()
                .and_then(|p| p.parse().ok())
                .unwrap_or(3000),
        }
    }

    pub fn addr(&self) -> String {
        format!("0.0.0.0:{}", self.port)
    }
}
