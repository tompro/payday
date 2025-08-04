use std::sync::{Arc, OnceLock};

use payday_core::api::invoice_api::InvoiceServiceApi;
use twelf::{Layer, config};

pub mod routes;

#[derive(Clone)]
pub struct AppState {
    pub config: Conf,
    pub invoice_service: Arc<dyn InvoiceServiceApi>,
}

#[config]
#[derive(Debug, Clone)]
pub struct Conf {
    pub psk: String,
}

pub fn load_env_config() -> &'static Conf {
    static INSTANCE: OnceLock<Conf> = OnceLock::new();
    INSTANCE.get_or_init(|| {
        Conf::with_layers(&[Layer::Env(Some("SERVICE_".to_owned()))]).unwrap_or_else(|e| {
            panic!("Failed to load configuration from environment variables: {e}")
        })
    })
}
