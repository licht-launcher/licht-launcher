//! Concurrent downloads and checksum verification.

pub(crate) mod checksum;
pub(crate) mod control;
pub(crate) mod download;
pub(crate) mod installation;
pub(crate) mod jobs;
pub(crate) mod manager;
mod pipeline;
pub(crate) mod verification;
