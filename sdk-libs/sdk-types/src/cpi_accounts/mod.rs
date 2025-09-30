mod config;
pub mod v1;
#[cfg(feature = "v2")]
pub mod v2;

pub use config::CpiAccountsConfig;
