//! What every command needs: where the app keeps its files, its config and the language.

use tokenradar_core::config::AppConfig;
use tokenradar_core::paths::Paths;
use tokenradar_i18n::{Domain, Localizer};

pub struct Context {
    /// `None` if no data directory can be found; then nothing is spooled or read.
    pub paths: Option<Paths>,
    pub config: AppConfig,
    pub l10n: Localizer,
    /// The app's pipe for this user.
    pub pipe_name: Option<String>,
}

impl Context {
    pub fn load() -> Self {
        let paths = Paths::resolve().ok();
        // A broken config must not break the statusline: fall back to the defaults.
        let config = paths
            .as_ref()
            .and_then(|p| AppConfig::load(p.config_path()).ok())
            .unwrap_or_default();
        let l10n = Localizer::new(config.general.locale(), &[Domain::Cli]);
        Self {
            paths,
            config,
            l10n,
            pipe_name: tokenradar_core::ipc::pipe_name().ok(),
        }
    }
}
