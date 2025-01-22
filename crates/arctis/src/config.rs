use config::{Config, ConfigError, Environment, File};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct RpcConfig {
    pub solana_rpc_url: String,
    pub solana_ws_url: String,
}

#[derive(Debug, Deserialize)]
pub struct Settings {
    pub rpc: RpcConfig,
}

impl Settings {
    fn new() -> Result<Self, ConfigError> {
        // Use ConfigBuilder instead of merge
        let settings = Config::builder()
            // Add the config file as a source
            .add_source(File::with_name("config"))
            // Optionally add environment variables with a prefix
            .add_source(Environment::with_prefix("APP"))
            .build()?; // Build the config

        // Try to deserialize the settings into the Settings struct
        settings.try_deserialize()
    }
}

pub fn get_settings() -> Result<Settings, ConfigError> {
    // Load settings
    match Settings::new() {
        Ok(settings) => {
            // println!("Loaded settings: {:?}", settings);
            Ok(settings)
        }
        Err(e) => {
            println!("Failed to load settings: {:?}", e);
            Err(e)
        }
    }
}
