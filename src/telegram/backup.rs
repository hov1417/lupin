use serde::{Deserialize, Serialize};

use crate::telegram::message::Message;

#[derive(Debug, Serialize, Deserialize)]
pub enum DialogType {
    User,
    Group,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DialogBackup {
    pub name: String,
    pub username: Option<String>,
    pub last_name: Option<String>,
    pub messages: Vec<Message>,
    pub dialog_type: DialogType,
}

impl DialogBackup {
    pub fn is_empty(&self) -> bool {
        self.messages.is_empty()
    }
}
