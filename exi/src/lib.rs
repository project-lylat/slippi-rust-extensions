//! This module houses the `SlippiEXIDevice`, which is in effect a "shadow subclass" of the C++
//! Slippi EXI device.
//!
//! What this means is that the Slippi EXI Device (C++) holds a pointer to the Rust
//! `SlippiEXIDevice` and forwards calls over the C FFI. This has a fairly clean mapping to "when
//! Slippi stuff is happening" and enables us to let the Rust side live in its own world.

use dolphin_integrations::Log;
use slippi_game_reporter::GameReporter;
use slippi_gg_api::APIClient;
use slippi_jukebox::Jukebox;
use slippi_matchmaking::MatchmakingClient;
use slippi_replay_uploader::ReplayUploader;
use slippi_user::UserManager;

mod config;
pub use config::{Config, FilePathsConfig, ReplayUploadConfig, SCMConfig};
pub use slippi_gg_api::Endpoints;
pub use slippi_replay_uploader::UploadJob;
pub use slippi_user::AccountFile;

/// An EXI Device subclass specific to managing and interacting with the game itself.
#[derive(Debug)]
pub struct SlippiEXIDevice {
    config: Config,
    pub game_reporter: GameReporter,
    pub user_manager: UserManager,
    pub matchmaking: MatchmakingClient,
    pub jukebox: Option<Jukebox>,
    pub replay_uploader: ReplayUploader,
}

pub enum JukeboxConfiguration {
    Start {
        initial_dolphin_system_volume: u8,
        initial_dolphin_music_volume: u8,
    },
    Stop,
}

impl SlippiEXIDevice {
    /// Creates and returns a new `SlippiEXIDevice` with default values.
    ///
    /// At the moment you should never need to call this yourself.
    pub fn new(config: Config) -> Self {
        tracing::info!(target: Log::SlippiOnline, "Starting SlippiEXIDevice");

        let api_client = APIClient::new(&config.scm.slippi_semver).with_endpoints(config.endpoints.clone());

        let user_manager = UserManager::new(
            api_client.clone(),
            config.paths.user_config_folder.clone().into(),
            config.scm.slippi_semver.clone(),
        )
        .with_account_files(config.account_files.clone());

        let game_reporter = GameReporter::new(
            api_client.clone(),
            user_manager.clone(),
            config.paths.iso.clone(),
            config.paths.user_config_folder.clone().into(),
        );

        // Playback has no need to deal with this.
        // (We could maybe silo more?)
        #[cfg(not(feature = "playback"))]
        user_manager.watch_for_login();

        let matchmaking = MatchmakingClient::new(user_manager.clone(), config.scm.slippi_semver.clone());

        let replay_uploader = match &config.replay_upload {
            Some(upload) => ReplayUploader::new(
                slippi_gg_api::user_agent(&config.scm.slippi_semver),
                Some(upload.web_root.clone()),
                upload.account_path.clone(),
            ),
            None => ReplayUploader::new(String::new(), None, Default::default()),
        };

        Self {
            config,
            game_reporter,
            user_manager,
            matchmaking,
            jukebox: None,
            replay_uploader,
        }
    }

    /// Stubbed for now, but this would get called by the C++ EXI device on DMAWrite.
    pub fn dma_write(&mut self, _address: usize, _size: usize) {}

    /// Stubbed for now, but this would get called by the C++ EXI device on DMARead.
    pub fn dma_read(&mut self, _address: usize, _size: usize) {}

    /// Configures a new Jukebox, or ensures an existing one is dropped if it's being disabled.
    pub fn configure_jukebox(&mut self, config: JukeboxConfiguration) {
        if let JukeboxConfiguration::Stop = config {
            self.jukebox = None;
            return;
        }

        if self.jukebox.is_some() {
            tracing::warn!(target: Log::SlippiOnline, "Jukebox is already active");
            return;
        }

        if let JukeboxConfiguration::Start {
            initial_dolphin_system_volume,
            initial_dolphin_music_volume,
        } = config
        {
            match Jukebox::new(
                self.config.paths.iso.clone(),
                initial_dolphin_system_volume,
                initial_dolphin_music_volume,
            ) {
                Ok(jukebox) => {
                    self.jukebox = Some(jukebox);
                },

                Err(e) => tracing::error!(
                    target: Log::SlippiOnline,
                    error = ?e,
                    "Failed to start Jukebox"
                ),
            }
        }
    }
}
