use crate::telegram::message::Message;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct ChatBackup {
    pub first_name: String,
    pub username: Option<String>,
    pub last_name: Option<String>,
    pub messages: Vec<Message>,
}
