use std::env;
use std::sync::OnceLock;

mod development;
mod production;

pub(crate) static _SLP_CFG: OnceLock<SlippiConfig> = OnceLock::new();

/// Lylat service settings for one environment. `None` keeps Slippi's value for that setting.
/// The Lylat matchmaking server and site are Lylat-only, so every environment sets them.
#[derive(Clone, Debug)]
pub struct SlippiConfig {
    /// GraphQL endpoint: game reports, match status and rank.
    pub graphql_url: Option<String>,

    /// User REST endpoint; the client appends `[-beta]/{uid}?...` to it.
    pub user_rest_url: Option<String>,

    /// Sign-in page; the client appends `?path={account file}`.
    pub login_page_url: Option<String>,

    /// Page opened when the client needs an update.
    pub update_page_url: Option<String>,

    /// The Lylat site, for the QR device sign-in (`/api/device/...`).
    pub origin: Option<String>,

    /// Lylat matchmaking server (ENet).
    pub mm_host: Option<String>,
    pub mm_port: Option<u16>,
}

impl SlippiConfig {
    /// Merges two configurations. Values in `other` take precedence.
    fn merge(self, other: Self) -> Self {
        Self {
            graphql_url: other.graphql_url.or(self.graphql_url),
            user_rest_url: other.user_rest_url.or(self.user_rest_url),
            login_page_url: other.login_page_url.or(self.login_page_url),
            update_page_url: other.update_page_url.or(self.update_page_url),
            origin: other.origin.or(self.origin),
            mm_host: other.mm_host.or(self.mm_host),
            mm_port: other.mm_port.or(self.mm_port),
        }
    }

    /// Overrides from the environment, read when the game starts, so a URL can be switched
    /// without a rebuild.
    fn from_env_vars() -> Self {
        Self {
            graphql_url: env::var("SLIPPI_GRAPHQL_URL").ok(),
            user_rest_url: env::var("SLIPPI_USER_REST_URL").ok(),
            login_page_url: env::var("SLIPPI_LOGIN_PAGE_URL").ok(),
            update_page_url: env::var("SLIPPI_UPDATE_PAGE_URL").ok(),
            origin: env::var("SLIPPI_LYLAT_ORIGIN").ok(),
            mm_host: env::var("SLIPPI_MM_HOST").ok(),
            mm_port: env::var("SLIPPI_MM_PORT").ok().and_then(|port| port.parse().ok()),
        }
    }

    #[cfg(feature = "slippi_env_development")]
    fn read_file_config(_env: &str) -> SlippiConfig {
        SlippiConfig::development()
    }

    #[cfg(feature = "slippi_env_production")]
    fn read_file_config(_env: &str) -> SlippiConfig {
        SlippiConfig::production()
    }

    /// Initializes the configuration based on a provided environment.
    fn init_config(env: &str) {
        let config: Self = Self::read_file_config(env);
        // Environment variables win over the built-in environment values.
        let merged_config = config.merge(Self::from_env_vars());
        tracing::warn!(
            "SlippiRustExtensions Config {:?} Environment {}",
            &merged_config,
            SlippiConfig::get_env()
        );
        let _ = _SLP_CFG.set(merged_config);
    }

    /// The environment this build was made for (`SLIPPI_ENV` at build time), "development" by default.
    pub const fn get_env() -> &'static str {
        match option_env!("SLIPPI_ENV") {
            None => "development",
            Some(env) => env,
        }
    }

    /// Retrieves the configuration. Initializes it if it's accessed for the first time.
    pub fn get() -> Self {
        if _SLP_CFG.get().is_none() {
            Self::init_config(Self::get_env());
        }

        _SLP_CFG.get().unwrap().clone()
    }
}
