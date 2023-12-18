use oauth2::basic::BasicClient;

use crate::db::Pool;

pub struct AppData {
    pub oauth2_client: Option<BasicClient>,
    pub db: Pool,
}
