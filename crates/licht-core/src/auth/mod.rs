//! Microsoft account authentication (OAuth device code → Xbox → Minecraft).

mod accounts;
mod client_id;
mod endpoints;
mod flow;
mod keyring_store;
mod launch;
mod parse;
mod types;

pub use accounts::{
    begin_login, get_active_profile, list_accounts, logout_account, poll_login, session_for_launch,
    set_active_account,
};
pub use client_id::{EMBEDDED_MSA_CLIENT_ID, MSA_CLIENT_ID_ENV, load_env_files, msa_client_id};
pub use launch::prepare_microsoft_launch;
pub use types::{AccountProfile, DeviceCodeChallenge, MicrosoftSession};
