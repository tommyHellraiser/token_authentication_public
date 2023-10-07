use std::fs::File;
use std::io::Error;
use std::io::ErrorKind::InvalidData;
use serde::Deserialize;
use tokio::sync::RwLock;
use crate::config::ENVIRONMENT_CONFIG;

pub struct EnvironmentConfig {
    config: RwLock<EnvironmentConfigInner>
}

#[derive(Deserialize)]
struct EnvironmentConfigInner {
    service_url: String,
    service_port: String,
    db_url: String,
    reset_db: bool
}

impl EnvironmentConfig {
    pub(super) fn new() -> Self {
        Self {
            config: RwLock::new(Self::load().unwrap())
        }
    }

    fn load() -> std::io::Result<EnvironmentConfigInner> {
        let file = File::open("config/env.json")
            .map_err(|e| Error::new(InvalidData, format!("{}", e)))?;

        match serde_json::from_reader::<_, EnvironmentConfigInner>(file) {
            Ok(config) => {Ok(config)},
            Err(e) => Err(Error::new(InvalidData, format!("{}", e)))?
        }
    }

    pub fn instance() -> &'static Self {
        &ENVIRONMENT_CONFIG
    }

    pub async fn get_service_url(&self) -> String {
        self.config.read().await.service_url.clone()
    }

    pub async fn get_service_port(&self) -> String {
        self.config.read().await.service_port.clone()
    }

    pub async fn get_db_url(&self) -> String {
        self.config.read().await.db_url.clone()
    }
    
    pub async fn reset_db(&self) -> bool {
        self.config.read().await.reset_db
    }
}