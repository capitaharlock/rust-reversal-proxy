use std::env;
use dotenv::dotenv;

#[derive(Clone, Debug)]
pub struct Config {
    pub database_url: String,
    pub port: u16,
    pub ws_port: u16,
    pub target_http_url: String,
    pub target_https_url: String,
    pub target_ws_url: String,
    pub http_requests_per_minute: u32,
    pub ws_connections_per_minute: u32,
    pub redis_url: String,
}

impl Config {
    pub fn from_env() -> Result<Self, ConfigError> {
        dotenv().ok();

        Ok(Config {
            database_url: env::var("DATABASE_URL")?,
            port: parse_env_var("SERVER_PORT")?,
            ws_port: parse_env_var("WS_PORT")?,
            target_http_url: env::var("TARGET_HTTP_URL")?,
            target_https_url: env::var("TARGET_HTTPS_URL")?,
            target_ws_url: env::var("TARGET_WS_URL")?,
            http_requests_per_minute: parse_env_var("HTTP_REQUESTS_PER_MINUTE")?,
            ws_connections_per_minute: parse_env_var("WS_CONNECTIONS_PER_MINUTE")?,
            redis_url: env::var("REDIS_URL")?,
        })
    }
}

fn parse_env_var<T: std::str::FromStr>(key: &str) -> Result<T, ConfigError>
where
    T::Err: std::fmt::Debug,
{
    env::var(key)?
        .parse()
        .map_err(|e| ConfigError::ParseError(key.to_string(), format!("{:?}", e)))
}

#[derive(Debug)]
pub enum ConfigError {
    EnvVarMissing(env::VarError),
    ParseError(String, String),
}

impl From<env::VarError> for ConfigError {
    fn from(err: env::VarError) -> Self {
        ConfigError::EnvVarMissing(err)
    }
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            ConfigError::EnvVarMissing(err) => write!(f, "Environment variable error: {}", err),
            ConfigError::ParseError(key, err) => write!(f, "Failed to parse {}: {}", key, err),
        }
    }
}

impl std::error::Error for ConfigError {}