//! Microsoft account authentication (OAuth browser + PKCE → Xbox → Minecraft).

mod accounts;
mod browser;
mod client_id;
mod endpoints;
mod flow;
mod keyring_store;
mod launch;
mod parse;
mod types;

pub use accounts::{
    get_active_profile, list_accounts, login_with_browser, logout_account, session_for_launch,
    set_active_account,
};
pub use client_id::{EMBEDDED_MSA_CLIENT_ID, MSA_CLIENT_ID_ENV, load_env_files, msa_client_id};
pub use launch::prepare_microsoft_launch;
pub use types::{AccountProfile, MicrosoftSession};
