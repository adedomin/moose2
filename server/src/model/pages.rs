use serde::Serialize;

use super::moose::Moose;

#[derive(Debug, Serialize, Clone)]
pub struct MooseSearch {
    /// The actual Moose page this moose belongs to.
    pub page: usize,
    pub moose: Moose,
}

#[derive(Debug, Serialize)]
pub struct MooseSearchPage {
    /// number of pages returned by query set (max: 10)
    pub pages: usize,
    pub result: Vec<MooseSearch>,
}

impl Default for MooseSearchPage {
    fn default() -> Self {
        MooseSearchPage {
            pages: 0,
            result: vec![],
        }
    }
}
