use std::io;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum CoreError {
    #[error("HTTP request failed")]
    Http(#[from] reqwest::Error),
    #[error("I/O operation failed")]
    Io(#[from] io::Error),
    #[error("version manifest JSON is invalid")]
    Manifest(#[from] serde_json::Error),
    #[error("version JSON is invalid")]
    Version(#[source] serde_json::Error),
    #[error("version JSON must contain either minecraftArguments or arguments")]
    VersionArguments,
    #[error("asset index JSON is invalid")]
    AssetIndex(#[source] serde_json::Error),
    #[error("downloaded SHA1 does not match")]
    Sha1Mismatch { expected: String, actual: String },
    #[error("downloaded MD5 does not match")]
    Md5Mismatch { expected: String, actual: String },
    #[error("downloaded SHA256 does not match")]
    Sha256Mismatch { expected: String, actual: String },
    #[error("downloaded SHA512 does not match")]
    Sha512Mismatch { expected: String, actual: String },
    #[error("download needs at least one attempt")]
    DownloadAttempts,
    #[error("download is paused")]
    DownloadPaused,
    #[error("download was cancelled")]
    DownloadCancelled,
    #[error("download response is invalid: {detail}")]
    DownloadResponse { detail: &'static str },
    #[error("download concurrency must be at least 1")]
    DownloadConcurrency,
    #[error("write concurrency must be at least 1")]
    WriteConcurrency,
    #[error("could not resolve the Licht data directory")]
    DataDirectory,
    #[error("settings JSON is invalid")]
    Settings(#[source] serde_json::Error),
    #[error("settings schema {schema} is not supported")]
    SettingsSchema { schema: u32 },
    #[error("download concurrency must be from 1 to 16")]
    SettingsConcurrency,
    #[error("write concurrency must be from 1 to 16")]
    SettingsWriteConcurrency,
    #[error("color theme must be dark")]
    SettingsTheme,
    #[error("memory must be from 512 to 16384 in steps of 256")]
    SettingsMemory,
    #[error("window size must be at least 1")]
    SettingsWindow,
    #[error("application directory must be an absolute path")]
    SettingsDirectory,
    #[error("java path is invalid")]
    SettingsJava,
    #[error("cache path is invalid")]
    CachePath,
    #[error("instance JSON is invalid")]
    Instance(#[source] serde_json::Error),
    #[error("instance schema {schema} is not supported")]
    InstanceSchema { schema: u32 },
    #[error("instance memory is invalid")]
    InstanceMemory,
    #[error("instance name is invalid")]
    InstanceName,
    #[error("an instance folder already exists")]
    InstanceExists,
    #[error("instance preparation is incomplete")]
    InstancePreparing,
    #[error("Java runtime index JSON is invalid")]
    JavaRuntime(#[source] serde_json::Error),
    #[error("Java runtime was not found for this system")]
    JavaRuntimeMissing,
    #[error("custom Java executable was not found")]
    JavaPath,
    #[error("Java version could not be read")]
    JavaProbe,
    #[error("Java {found} does not match required major version {required}")]
    JavaMajor { found: u32, required: u32 },
    #[error("native archive could not be read")]
    NativeArchive(#[source] zip::result::ZipError),
    #[error("native archive path is invalid")]
    NativePath,
    #[error("game command is empty")]
    GameCommand,
    #[error("offline username is empty")]
    OfflineName,
    #[error("command arguments are incomplete")]
    LaunchArgs,
    #[error("version was not found")]
    VersionMissing,
    #[error("version manifest is unavailable")]
    VersionManifestUnavailable,
    #[error("this system is not supported for launch")]
    LaunchHost,
    #[error("database operation failed")]
    Database(#[from] sqlx::Error),
    #[error("database schema {schema} is not supported")]
    DatabaseSchema { schema: u32 },
    #[error("database service failed: {detail}")]
    DatabaseService { detail: &'static str },
    #[error("download job data is invalid: {detail}")]
    DownloadJobInvalid { detail: &'static str },
    #[error("download job JSON is invalid")]
    DownloadJobJson(#[source] serde_json::Error),
    #[error("download job was not found")]
    DownloadJobMissing,
    #[error("an unfinished download job already exists for this instance")]
    DownloadJobExists,
    #[error("download job state transition is invalid")]
    DownloadJobTransition,
    #[error("{detail}")]
    DownloadJobExecution { detail: String },
    #[error("LICHT_MSA_CLIENT_ID is missing or empty")]
    MsaClientIdMissing,
    #[error("Microsoft auth JSON is invalid")]
    AuthJson(#[source] serde_json::Error),
    #[error("Microsoft token response is invalid")]
    AuthToken,
    #[error("Microsoft sign-in timed out")]
    AuthBrowserTimeout,
    #[error("could not open the system browser for Microsoft sign-in")]
    AuthBrowserOpen,
    #[error("Microsoft sign-in state mismatch")]
    AuthStateMismatch,
    #[error("Microsoft sign-in was declined")]
    AuthDeclined,
    #[error("Microsoft OAuth failed: {error}")]
    AuthOAuth { error: String, description: String },
    #[error("Xbox Live authentication failed")]
    AuthXbox,
    #[error("this Microsoft account is banned from Xbox Live")]
    AuthXboxBanned,
    #[error("this Microsoft account has no Xbox profile")]
    AuthXboxNoAccount,
    #[error("Xbox Live is unavailable for this account's country")]
    AuthXboxUnavailable,
    #[error("this Microsoft account needs adult verification on Xbox")]
    AuthXboxAdultVerification,
    #[error("this Microsoft account is a child and needs to join a Family")]
    AuthXboxChild,
    #[error("Minecraft services rejected this app (not allow-listed yet)")]
    AuthAppNotAllowlisted,
    #[error("Minecraft authentication failed")]
    AuthMinecraft,
    #[error("Minecraft profile was not found for this account")]
    AuthProfileMissing,
    #[error("this Microsoft account does not own Minecraft: Java Edition")]
    AuthNoOwnership,
    #[error("no signed-in Microsoft account")]
    AuthNotSignedIn,
    #[error("secure credential storage failed")]
    AuthKeyring,
    #[error("mod loader metadata failed: {detail}")]
    LoaderMeta { detail: String },
    #[error("mod loader version was not found")]
    LoaderVersionMissing,
    #[error("mod loader JSON is invalid")]
    LoaderJson(#[source] serde_json::Error),
    #[error("mod loader installer archive is invalid")]
    LoaderArchive(#[source] zip::result::ZipError),
    #[error("mod loader installation failed: {detail}")]
    LoaderInstall { detail: String },
    #[error("mod loader client processor {processor} failed (exit code {code:?})")]
    LoaderProcessor { processor: usize, code: Option<i32> },
    #[error("modpack metadata failed: {detail}")]
    ModpackMeta { detail: String },
    #[error("LICHT_CURSEFORGE_API_KEY is missing or empty")]
    CurseforgeKeyMissing,
    #[error("curseforge download host is not allowed")]
    CurseforgeDownloadHost,
    #[error("curseforge pack disallows third-party distribution")]
    CurseforgePackDistributionDenied,
    #[error("curseforge mods disallow third-party distribution: {mods}")]
    CurseforgeDistributionDenied { mods: String },
    #[error("curseforge file was not found: {file_id}")]
    CurseforgeFileMissing { file_id: u64 },
    #[error("modpack archive is invalid")]
    ModpackArchive,
}

#[cfg(test)]
mod tests {
    use std::error::Error;
    use std::io;

    #[test]
    fn propagating_an_io_error_preserves_its_cause_and_category() {
        fn operation() -> crate::Result<()> {
            Err(io::Error::from(io::ErrorKind::PermissionDenied))?;
            Ok(())
        }

        let error = operation().expect_err("the operation should fail");
        let cause = error
            .source()
            .expect("the original cause should remain available")
            .downcast_ref::<io::Error>()
            .expect("the cause should remain an I/O error");

        assert_eq!(cause.kind(), io::ErrorKind::PermissionDenied);
        assert_eq!(error.to_string(), "I/O operation failed");
    }
}
