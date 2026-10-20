use std::fs;
use std::path::Path;

use crate::Result;
use crate::hashes::sha1_hex;
use crate::store::repositories::content_fingerprints::ContentFingerprint;

pub(super) fn content_fingerprint(
    path: &Path,
    relative_path: String,
    previous: Option<&ContentFingerprint>,
    refresh: bool,
) -> Result<ContentFingerprint> {
    let before = fs::metadata(path)?;
    let size = i64::try_from(before.len()).map_err(std::io::Error::other)?;
    let modified = modification_stamp(&before)?;
    if !refresh
        && let Some(previous) = previous
        && previous.relative_path == relative_path
        && previous.size == size
        && previous.modified == modified
    {
        return Ok(previous.clone());
    }
    let sha1 = sha1_hex(&fs::read(path)?);
    let after = fs::metadata(path)?;
    if before.len() != after.len() || modified != modification_stamp(&after)? {
        return Err(std::io::Error::other(
            "content file changed while calculating its fingerprint",
        )
        .into());
    }
    Ok(ContentFingerprint {
        relative_path,
        size,
        modified,
        sha1,
    })
}

fn modification_stamp(metadata: &fs::Metadata) -> Result<String> {
    let modified = metadata.modified()?;
    Ok(match modified.duration_since(std::time::UNIX_EPOCH) {
        Ok(duration) => format!("{}:{}", duration.as_secs(), duration.subsec_nanos()),
        Err(error) => format!(
            "-{}:{}",
            error.duration().as_secs(),
            error.duration().subsec_nanos()
        ),
    })
}
