use std::path::PathBuf;

use oauth2::basic::BasicClient;

use crate::db::Pool;

pub struct AppData {
    pub db: Pool,
    pub moose_dump: PathBuf,
    pub oauth2_client: Option<BasicClient>,
}
