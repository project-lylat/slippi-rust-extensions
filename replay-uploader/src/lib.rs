use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, SyncSender, TrySendError};
use std::thread;
use std::time::Duration;

use dolphin_integrations::Log;
use flate2::Compression;
use flate2::write::GzEncoder;
use serde_json::Value;
use sha2::{Digest, Sha256};
use ureq::{Agent, AgentBuilder};

const MAX_PENDING: usize = 4;
const MAX_GZIP_BYTES: usize = 20_000_000;
const MAX_RAW_BYTES: u64 = 80 * 1024 * 1024;
const MAX_FILE_NAME_CHARS: usize = 120;
const CONNECT_TIMEOUT: Duration = Duration::from_secs(4);
const FEATURES_TIMEOUT: Duration = Duration::from_secs(15);
const UPLOAD_TIMEOUT: Duration = Duration::from_secs(120);
const BOUNDARY: &str = "----LylatReplayUpload7f3a9c2e";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UploadJob {
    pub path: PathBuf,

    pub started_at: Option<String>,

    pub match_id: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Outcome {
    Sent,
    Duplicate,
    Skipped(String),
    Failed(String),
}

#[derive(Debug)]
pub struct ReplayUploader {
    sender: Option<SyncSender<UploadJob>>,
}

impl ReplayUploader {
    pub fn new(user_agent: String, api_root: Option<String>, account_path: PathBuf) -> Self {
        let Some(api_root) = api_root else {
            return Self { sender: None };
        };

        let (sender, receiver) = mpsc::sync_channel::<UploadJob>(MAX_PENDING);
        let agent = AgentBuilder::new()
            .user_agent(&user_agent)
            .redirects(0)
            .timeout_connect(CONNECT_TIMEOUT)
            .build();
        let api = format!("{}/api/v1", api_root.trim_end_matches('/'));

        let spawned = thread::Builder::new().name("SlippiReplayUploader".into()).spawn(move || {
            for job in receiver {
                let outcome = upload(&agent, &api, &account_path, &job);
                log_outcome(&job, &outcome);
            }
        });

        match spawned {
            Ok(_) => Self { sender: Some(sender) },
            Err(error) => {
                tracing::error!(target: Log::SlippiOnline, ?error, "Replay upload thread failed to start");
                Self { sender: None }
            },
        }
    }

    pub fn queue(&self, job: UploadJob) -> bool {
        let Some(sender) = &self.sender else {
            tracing::warn!(target: Log::SlippiOnline, path = ?job.path, "Replay upload skipped: no Lylat server configured");
            return false;
        };

        match sender.try_send(job) {
            Ok(()) => true,
            Err(TrySendError::Full(job)) => {
                tracing::warn!(target: Log::SlippiOnline, path = ?job.path, "Replay upload skipped: {MAX_PENDING} uploads already pending");
                false
            },
            Err(TrySendError::Disconnected(job)) => {
                tracing::error!(target: Log::SlippiOnline, path = ?job.path, "Replay upload skipped: upload thread stopped");
                false
            },
        }
    }
}

fn log_outcome(job: &UploadJob, outcome: &Outcome) {
    match outcome {
        Outcome::Sent => tracing::info!(target: Log::SlippiOnline, path = ?job.path, "Replay uploaded to Lylat"),
        Outcome::Duplicate => tracing::info!(target: Log::SlippiOnline, path = ?job.path, "Replay already on Lylat"),
        Outcome::Skipped(reason) => {
            tracing::info!(target: Log::SlippiOnline, path = ?job.path, "Replay upload skipped: {reason}")
        },
        Outcome::Failed(reason) => tracing::warn!(target: Log::SlippiOnline, path = ?job.path, "Replay upload failed: {reason}"),
    }
}

fn upload(agent: &Agent, api: &str, account_path: &Path, job: &UploadJob) -> Outcome {
    let Some((uid, play_key)) = read_account(account_path) else {
        return Outcome::Skipped("no Lylat account in lylat.json".into());
    };

    let features = agent
        .get(&format!("{api}/me/features"))
        .timeout(FEATURES_TIMEOUT)
        .set("X-Lylat-Uid", &uid)
        .set("X-Lylat-Key", &play_key)
        .set("Accept", "application/json")
        .call();

    let features = match features {
        Ok(response) => match response.into_json::<Value>() {
            Ok(features) => features,
            Err(error) => return Outcome::Skipped(format!("features unreadable: {error}")),
        },
        Err(ureq::Error::Status(402, _)) => return Outcome::Skipped("not subscribed".into()),
        Err(ureq::Error::Status(code, _)) => return Outcome::Skipped(format!("features check returned {code}")),
        Err(error) => return Outcome::Skipped(format!("features check failed: {error}")),
    };

    if let Err(reason) = should_upload(&features) {
        return Outcome::Skipped(reason);
    }

    match std::fs::metadata(&job.path) {
        Ok(metadata) if metadata.len() > MAX_RAW_BYTES => return Outcome::Skipped("replay over 80 MB".into()),
        Ok(_) => {},
        Err(error) => return Outcome::Failed(format!("replay unreadable: {error}")),
    }

    let raw = match std::fs::read(&job.path) {
        Ok(raw) => raw,
        Err(error) => return Outcome::Failed(format!("replay unreadable: {error}")),
    };

    let sha = sha256_hex(&raw);
    let gz = match gzip(&raw) {
        Ok(gz) => gz,
        Err(error) => return Outcome::Failed(format!("gzip failed: {error}")),
    };

    if gz.len() > MAX_GZIP_BYTES {
        return Outcome::Skipped("compressed replay over 20,000,000 bytes".into());
    }

    let file_name = job
        .path
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| name.ends_with(".slp") && name.chars().count() <= MAX_FILE_NAME_CHARS);

    let body = multipart_body(
        BOUNDARY,
        &sha,
        job.started_at.as_deref(),
        job.match_id.as_deref(),
        file_name,
        &gz_file_name(job.started_at.as_deref(), &sha),
        &gz,
    );

    let response = agent
        .post(&format!("{api}/replays"))
        .timeout(UPLOAD_TIMEOUT)
        .set("X-Lylat-Uid", &uid)
        .set("X-Lylat-Key", &play_key)
        .set("Accept", "application/json")
        .set("Content-Type", &format!("multipart/form-data; boundary={BOUNDARY}"))
        .send_bytes(&body);

    match response {
        Ok(response) => classify(response.status(), ""),
        Err(ureq::Error::Status(code, response)) => classify(code, &response.into_string().unwrap_or_default()),
        Err(error) => Outcome::Failed(error.to_string()),
    }
}

// The Lylat account at the top of lylat.json, never the linked Slippi one.
fn read_account(path: &Path) -> Option<(String, String)> {
    let value: Value = serde_json::from_str(&std::fs::read_to_string(path).ok()?).ok()?;
    let field = |key: &str| {
        value
            .get(key)
            .and_then(Value::as_str)
            .filter(|field| !field.is_empty())
            .map(String::from)
    };

    Some((field("uid")?, field("playKey")?))
}

fn should_upload(features: &Value) -> Result<(), String> {
    let flag = |key: &str| features.get(key).and_then(Value::as_bool).unwrap_or(false);

    if !flag("subscribed") {
        return Err("not subscribed".into());
    }

    if !flag("autoUpload") {
        return Err("auto upload is off for this account".into());
    }

    let bytes = |key: &str| features.get(key).and_then(Value::as_u64);
    if let (Some(quota), Some(used)) = (bytes("quotaBytes"), bytes("usedBytes")) {
        if used >= quota {
            return Err("replay storage is full".into());
        }
    }

    Ok(())
}

fn classify(status: u16, body: &str) -> Outcome {
    match status {
        200..=299 => Outcome::Sent,
        409 => Outcome::Duplicate,
        _ => {
            let message = serde_json::from_str::<Value>(body)
                .ok()
                .and_then(|error| {
                    let code = error.get("error").and_then(Value::as_str)?.to_string();
                    let message = error.get("message").and_then(Value::as_str).unwrap_or_default();
                    Some(if message.is_empty() {
                        code
                    } else {
                        format!("{code}: {message}")
                    })
                })
                .unwrap_or_else(|| String::from("no error body"));

            Outcome::Failed(format!("HTTP {status} ({message})"))
        },
    }
}

fn sha256_hex(data: &[u8]) -> String {
    Sha256::digest(data).iter().map(|byte| format!("{byte:02x}")).collect()
}

fn gzip(data: &[u8]) -> std::io::Result<Vec<u8>> {
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(data)?;
    encoder.finish()
}

fn gz_file_name(started_at: Option<&str>, sha: &str) -> String {
    match started_at {
        Some(started_at) if !started_at.is_empty() => format!("{started_at}.slp.gz"),
        _ => format!("{}.slp.gz", &sha[..16]),
    }
}

// The API expects the file field last.
fn multipart_body(
    boundary: &str,
    sha: &str,
    started_at: Option<&str>,
    match_id: Option<&str>,
    file_name: Option<&str>,
    gz_name: &str,
    gz: &[u8],
) -> Vec<u8> {
    let mut body = Vec::with_capacity(gz.len() + 1024);
    let mut field = |name: &str, value: &str| {
        body.extend_from_slice(
            format!("--{boundary}\r\nContent-Disposition: form-data; name=\"{name}\"\r\n\r\n{value}\r\n").as_bytes(),
        );
    };

    field("sha256", sha);
    if let Some(started_at) = started_at.filter(|value| !value.is_empty()) {
        field("startedAt", started_at);
    }
    if let Some(match_id) = match_id.filter(|value| !value.is_empty()) {
        field("matchId", match_id);
    }
    if let Some(file_name) = file_name {
        field("fileName", file_name);
    }

    body.extend_from_slice(
        format!(
            "--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"{gz_name}\"\r\nContent-Type: application/gzip\r\n\r\n"
        )
        .as_bytes(),
    );
    body.extend_from_slice(gz);
    body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());
    body
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::io::Read;

    #[test]
    fn sha256_matches_known_vector() {
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn gzip_round_trips() {
        let gz = gzip(b"replay bytes").unwrap();
        let mut out = String::new();
        flate2::read::GzDecoder::new(&gz[..]).read_to_string(&mut out).unwrap();
        assert_eq!(out, "replay bytes");
    }

    #[test]
    fn gz_name_uses_start_time_or_sha() {
        let sha = "0123456789abcdef0123456789abcdef";
        assert_eq!(gz_file_name(Some("2026-10-26T12:00:00Z"), sha), "2026-10-26T12:00:00Z.slp.gz");
        assert_eq!(gz_file_name(None, sha), "0123456789abcdef.slp.gz");
        assert_eq!(gz_file_name(Some(""), sha), "0123456789abcdef.slp.gz");
    }

    #[test]
    fn uploads_only_when_subscribed_with_auto_upload_and_room() {
        assert!(should_upload(&json!({"subscribed": true, "autoUpload": true})).is_ok());
        assert!(should_upload(&json!({"subscribed": true, "autoUpload": true, "quotaBytes": 10, "usedBytes": 3})).is_ok());
        assert!(should_upload(&json!({"subscribed": false, "autoUpload": true})).is_err());
        assert!(should_upload(&json!({"subscribed": true, "autoUpload": false})).is_err());
        assert!(should_upload(&json!({"subscribed": true})).is_err());
        assert!(should_upload(&json!({"subscribed": true, "autoUpload": true, "quotaBytes": 10, "usedBytes": 10})).is_err());
    }

    #[test]
    fn statuses_map_to_outcomes() {
        assert_eq!(classify(201, ""), Outcome::Sent);
        assert_eq!(classify(409, ""), Outcome::Duplicate);
        assert_eq!(
            classify(413, r#"{"error":"quota_full","message":"Storage is full"}"#),
            Outcome::Failed("HTTP 413 (quota_full: Storage is full)".into())
        );
        assert_eq!(classify(500, "oops"), Outcome::Failed("HTTP 500 (no error body)".into()));
    }

    #[test]
    fn multipart_has_fields_in_order_then_the_file() {
        let body = multipart_body(
            "B",
            "abc",
            Some("2026-10-26T12:00:00Z"),
            Some("mode.unranked-1"),
            Some("Game_1.slp"),
            "2026-10-26T12:00:00Z.slp.gz",
            b"GZ",
        );
        let expected = "--B\r\nContent-Disposition: form-data; name=\"sha256\"\r\n\r\nabc\r\n\
--B\r\nContent-Disposition: form-data; name=\"startedAt\"\r\n\r\n2026-10-26T12:00:00Z\r\n\
--B\r\nContent-Disposition: form-data; name=\"matchId\"\r\n\r\nmode.unranked-1\r\n\
--B\r\nContent-Disposition: form-data; name=\"fileName\"\r\n\r\nGame_1.slp\r\n\
--B\r\nContent-Disposition: form-data; name=\"file\"; filename=\"2026-10-26T12:00:00Z.slp.gz\"\r\nContent-Type: application/gzip\r\n\r\nGZ\r\n--B--\r\n";
        assert_eq!(String::from_utf8(body).unwrap(), expected);
    }

    #[test]
    fn multipart_leaves_out_missing_fields() {
        let body = String::from_utf8(multipart_body("B", "abc", None, Some(""), None, "x.slp.gz", b"GZ")).unwrap();
        assert!(!body.contains("startedAt"));
        assert!(!body.contains("matchId"));
        assert!(!body.contains("fileName"));
    }

    #[test]
    fn account_is_the_top_level_lylat_one() {
        let dir = std::env::temp_dir().join(format!("lylat-upload-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("lylat.json");
        std::fs::write(&path, r#"{"uid":"ly","playKey":"lk","slippi":{"uid":"slp","playKey":"sk"}}"#).unwrap();
        assert_eq!(read_account(&path), Some(("ly".into(), "lk".into())));
        std::fs::write(&path, r#"{"slippi":{"uid":"slp","playKey":"sk"}}"#).unwrap();
        assert_eq!(read_account(&path), None);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn queue_without_a_server_drops_the_job() {
        let uploader = ReplayUploader::new("UA".into(), None, PathBuf::from("lylat.json"));
        assert!(!uploader.queue(UploadJob {
            path: PathBuf::from("Game.slp"),
            started_at: None,
            match_id: None,
        }));
    }
}
