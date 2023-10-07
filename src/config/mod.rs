use lazy_static::lazy_static;
use crate::config::environment::EnvironmentConfig;

pub mod environment;
pub mod openssl;
pub mod shutdown;

lazy_static!(
    static ref ENVIRONMENT_CONFIG: EnvironmentConfig = EnvironmentConfig::new();
);