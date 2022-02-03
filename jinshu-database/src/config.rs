use jinshu_utils::secret::Secret;
use sea_orm::ConnectOptions;
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Deserialize, Serialize)]
pub struct DatabaseConfig {
    pub schema: String,
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: Secret,
    pub path: String,
    pub max_connections: u32,
    pub min_connections: u32,
    pub connect_timeout_ms: u64,
    pub idle_timeout_ms: u64,
    pub sqlx_log: bool,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            schema: "postgres".to_string(),
            host: "localhost".to_string(),
            port: 5432,
            username: "jinshu".to_string(),
            password: Secret::new("1qaz2wsx"),
            path: "jinshu".to_string(),
            max_connections: 16,
            min_connections: 10,
            connect_timeout_ms: 8000,
            idle_timeout_ms: 8000,
            sqlx_log: true,
        }
    }
}

impl DatabaseConfig {
    pub fn connection_url(&self) -> Secret {
        Secret::new(format!(
            "{}://{}:{}@{}:{}/{}",
            self.schema,
            self.username,
            self.password.expose(),
            self.host,
            self.port,
            self.path
        ))
    }
}

impl From<DatabaseConfig> for ConnectOptions {
    fn from(config: DatabaseConfig) -> Self {
        let connection_url = config.connection_url();

        let DatabaseConfig {
            max_connections,
            min_connections,
            connect_timeout_ms,
            idle_timeout_ms,
            sqlx_log,
            ..
        } = config;

        let mut opt = ConnectOptions::new(connection_url.expose().to_string());
        opt.max_connections(max_connections)
            .min_connections(min_connections)
            .connect_timeout(Duration::from_millis(connect_timeout_ms))
            .idle_timeout(Duration::from_millis(idle_timeout_ms))
            .sqlx_logging(sqlx_log);

        opt
    }
}

#[cfg(test)]
mod test {
    use super::DatabaseConfig;
    use sea_orm::ConnectOptions;

    #[test]
    fn default_into() {
        let _options: ConnectOptions = DatabaseConfig::default().into();
    }
}
