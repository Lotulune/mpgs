use crate::admin::hash_token;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use utoipa::ToSchema;

#[derive(Debug, Clone)]
pub struct ConfigFileManager {
    config_dir: PathBuf,
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PendingServiceIdentityRequest {
    pub service_name: String,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ConfigStateResponse {
    pub active_config_version: String,
    pub pending_config_version: Option<String>,
    pub restart_required: bool,
    pub last_startup_status: String,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PendingConfigResponse {
    pub pending_config_version: String,
    pub restart_required: bool,
}

#[derive(Debug, Deserialize)]
struct ActiveServiceConfig {
    bind_addr: Option<String>,
    service_identity: ActiveServiceIdentityConfig,
}

#[derive(Debug, Deserialize)]
struct ActiveServiceIdentityConfig {
    instance_id: String,
    version: Option<String>,
}

impl ConfigFileManager {
    pub fn new(config_dir: impl Into<PathBuf>) -> Self {
        Self {
            config_dir: config_dir.into(),
        }
    }

    pub fn state(&self) -> io::Result<ConfigStateResponse> {
        Ok(ConfigStateResponse {
            active_config_version: self.active_config_version()?,
            pending_config_version: self.pending_config_version()?,
            restart_required: pending_service_path(&self.config_dir).is_file(),
            last_startup_status: "ok".to_string(),
        })
    }

    pub fn write_pending_service_identity(
        &self,
        request: &PendingServiceIdentityRequest,
    ) -> io::Result<PendingConfigResponse> {
        let active_service = read_service_config(&active_service_path(&self.config_dir))?;
        let pending_dir = self.config_dir.join("pending");
        fs::create_dir_all(&pending_dir)?;

        let service_toml = format!(
            r#"bind_addr = "{bind_addr}"

[service_identity]
instance_id = "{instance_id}"
name = "{service_name}"
version = "{version}"
"#,
            bind_addr = escape_toml_string(
                active_service
                    .bind_addr
                    .as_deref()
                    .unwrap_or("0.0.0.0:4310")
            ),
            instance_id = escape_toml_string(&active_service.service_identity.instance_id),
            service_name = escape_toml_string(&request.service_name),
            version = escape_toml_string(
                active_service
                    .service_identity
                    .version
                    .as_deref()
                    .unwrap_or(env!("CARGO_PKG_VERSION"))
            )
        );
        atomic_write(&pending_service_path(&self.config_dir), &service_toml)?;

        Ok(PendingConfigResponse {
            pending_config_version: hash_token(&service_toml),
            restart_required: true,
        })
    }

    pub fn active_config_version(&self) -> io::Result<String> {
        let service_toml = fs::read_to_string(active_service_path(&self.config_dir))?;
        let secrets_toml = fs::read_to_string(active_secrets_path(&self.config_dir))?;
        Ok(hash_token(&format!("{service_toml}\n{secrets_toml}")))
    }

    fn pending_config_version(&self) -> io::Result<Option<String>> {
        let path = pending_service_path(&self.config_dir);
        if !path.is_file() {
            return Ok(None);
        }

        let service_toml = fs::read_to_string(path)?;
        Ok(Some(hash_token(&service_toml)))
    }
}

fn read_service_config(path: &Path) -> io::Result<ActiveServiceConfig> {
    let contents = fs::read_to_string(path)?;
    toml::from_str(&contents).map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))
}

fn active_service_path(config_dir: &Path) -> PathBuf {
    config_dir.join("active").join("service.toml")
}

fn active_secrets_path(config_dir: &Path) -> PathBuf {
    config_dir.join("active").join("secrets.toml")
}

fn pending_service_path(config_dir: &Path) -> PathBuf {
    config_dir.join("pending").join("service.toml")
}

fn atomic_write(path: &Path, contents: &str) -> io::Result<()> {
    let temp_path = path.with_extension("tmp");
    fs::write(&temp_path, contents)?;
    fs::rename(temp_path, path)
}

fn escape_toml_string(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}
