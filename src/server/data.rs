use std::time::SystemTime;
use std::time::UNIX_EPOCH;

use serde::Serialize;

pub type NameHistory = Vec<NameHistoryElement>;


#[derive(Debug, Serialize)]
pub struct NameHistoryElement {
    
    pub name: String,
    
    #[serde(rename = "changedToAt")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub changed_to_at: Option<u128>,
}

impl NameHistoryElement {

    pub fn new(name: String) -> Self {
        Self { name, changed_to_at: None }
    }

    pub fn new_full(name: String, changed_to_at: u128) -> Self {
        Self { name, changed_to_at: Some(changed_to_at) }
    }
    
    pub fn new_current(name: String) -> Self {
        let changed_to_at = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis();
        Self { name, changed_to_at: Some(changed_to_at) }
    }
}