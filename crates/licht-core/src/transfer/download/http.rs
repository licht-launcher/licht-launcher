use super::TransferJob;
use crate::{CoreError, Result};
use reqwest::StatusCode;
use reqwest::header::{ACCEPT_ENCODING, CONTENT_ENCODING, CONTENT_RANGE, RANGE};

pub(super) async fn request(job: &TransferJob<'_>, offset: u64) -> Result<reqwest::Response> {
    let mut request = job.client.get(job.url).header(ACCEPT_ENCODING, "identity");
    if offset > 0 {
        request = request.header(RANGE, format!("bytes={offset}-"));
    }
    tokio::select! { biased; error = job.control.interrupted() => Err(error), response = request.send() => Ok(response?), }
}

pub(super) fn response_total(
    response: &reqwest::Response,
    offset: u64,
    published_size: u64,
) -> Result<Option<u64>> {
    if let Some(encoding) = response.headers().get(CONTENT_ENCODING)
        && !encoding.as_bytes().eq_ignore_ascii_case(b"identity")
    {
        return Err(invalid_response("encoded bodies cannot be resumed"));
    }
    let total = match response.status() {
        StatusCode::OK => response.content_length(),
        StatusCode::PARTIAL_CONTENT => {
            let range = response
                .headers()
                .get(CONTENT_RANGE)
                .and_then(|value| value.to_str().ok())
                .and_then(|value| value.strip_prefix("bytes "))
                .ok_or_else(|| invalid_response("missing or invalid Content-Range"))?;
            let (bounds, total) = range
                .split_once('/')
                .ok_or_else(|| invalid_response("invalid range total"))?;
            let (start, end) = bounds
                .split_once('-')
                .ok_or_else(|| invalid_response("invalid range bounds"))?;
            let parse = |value: &str| {
                value
                    .parse::<u64>()
                    .map_err(|_| invalid_response("invalid range number"))
            };
            let (start, end, total) = (parse(start)?, parse(end)?, parse(total)?);
            if start != offset || total <= start || end.checked_add(1) != Some(total) {
                return Err(invalid_response(
                    "range does not match the requested suffix",
                ));
            }
            if response
                .content_length()
                .is_some_and(|length| length != total - start)
            {
                return Err(invalid_response(
                    "range length does not match Content-Length",
                ));
            }
            Some(total)
        }
        _ => return Err(invalid_response("expected a full or partial file response")),
    };
    if published_size > 0 {
        if total.is_some_and(|total| total != published_size) {
            return Err(invalid_response(
                "response size does not match published size",
            ));
        }
        return Ok(Some(published_size));
    }
    Ok(total)
}

pub(super) fn invalid_response(detail: &'static str) -> CoreError {
    CoreError::DownloadResponse { detail }
}
