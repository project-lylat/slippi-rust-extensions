use slippi_gg_api::Endpoints;
use slippi_user::AccountFile;

/// Various paths that we need in a few places.
#[derive(Debug)]
pub struct FilePathsConfig {
    pub iso: String,
    pub user_config_folder: String,
}

/// Source control semver related parameters.
#[derive(Debug)]
pub struct SCMConfig {
    pub slippi_semver: String,
    // These can be re-enabled whenever they're needed.
    // pub desc_str: String,
    // pub branch_str: String,
    // pub rev_str: String,
    // pub rev_git_str: String,
    // pub rev_cache_str: String,
    // pub netplay_dolphin_ver: String,
    // pub distributor_str: String,
}

/// Core EXI device parameters that we need provided by the Dolphin side.
#[derive(Debug)]
pub struct Config {
    pub paths: FilePathsConfig,
    pub scm: SCMConfig,

    /// Servers the device talks to; Slippi's unless the Dolphin side passes others.
    pub endpoints: Endpoints,

    /// Lylat: account files to read, in order. Empty keeps Slippi's `user.json`.
    pub account_files: Vec<AccountFile>,
}
