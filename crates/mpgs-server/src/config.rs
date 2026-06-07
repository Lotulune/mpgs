use std::collections::HashMap;
use std::net::{AddrParseError, SocketAddr};

use crate::ServiceInfoConfig;
use thiserror::Error;

#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub bind_addr: SocketAddr,
    pub database_url: String,
    pub service_info: ServiceInfoConfig,
}

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("Missing required environment variable {0}")]
    MissingRequiredEnv(&'static str),
    #[error("MPGS_SERVER_BIND must be a valid socket address: {0}")]
    InvalidBindAddr(AddrParseError),
}

impl ServerConfig {
    pub fn from_env() -> Result<Self, ConfigError> {
        Self::from_env_vars(std::env::vars())
    }

    pub fn from_env_vars(
        vars: impl IntoIterator<Item = (String, String)>,
    ) -> Result<Self, ConfigError> {
        let vars: HashMap<String, String> = vars.into_iter().collect();
        let bind_addr = vars
            .get("MPGS_SERVER_BIND")
            .map(String::as_str)
            .unwrap_or("127.0.0.1:4310")
            .parse()
            .map_err(ConfigError::InvalidBindAddr)?;

        let database_url = vars
            .get("MPGS_DATABASE_URL")
            .cloned()
            .ok_or(ConfigError::MissingRequiredEnv("MPGS_DATABASE_URL"))?;

        let service_info = ServiceInfoConfig {
            service_instance_id: vars
                .get("MPGS_SERVICE_INSTANCE_ID")
                .cloned()
                .unwrap_or_else(|| "018fb770-8998-7699-a6e4-b7b59f2f9c01".to_string()),
            service_name: vars
                .get("MPGS_SERVICE_NAME")
                .cloned()
                .unwrap_or_else(|| "MPGS Public Discovery Service".to_string()),
            service_version: vars
                .get("MPGS_SERVICE_VERSION")
                .cloned()
                .unwrap_or_else(|| env!("CARGO_PKG_VERSION").to_string()),
        };

        Ok(Self {
            bind_addr,
            database_url,
            service_info,
        })
    }
}
