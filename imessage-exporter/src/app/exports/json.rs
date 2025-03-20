/*!
 JSON export functionality for iMessage data.
*/

use serde::Serialize;
use chrono::{DateTime, Utc};

use crate::app::error::RuntimeError;
use imessage_database::tables::messages::Message;

#[derive(Serialize)]
struct MessageJson {
    id: i32,
    message: String,
    sender: String,
    timestamp: String,
}

/// Export messages to JSON format
pub fn export_messages(messages: &[Message]) -> Result<String, RuntimeError> {
    let json_messages: Vec<MessageJson> = messages
        .iter()
        .map(|msg| MessageJson {
            id: msg.rowid,
            message: msg.text.clone().unwrap_or_default(),
            sender: if msg.is_from_me { "Me".to_string() } else { "Other".to_string() },
            timestamp: DateTime::<Utc>::from_timestamp(msg.date, 0)
                .map(|dt| dt.to_rfc3339())
                .unwrap_or_default(),
        })
        .collect();

    serde_json::to_string_pretty(&json_messages).map_err(|e| RuntimeError::CreateError(e.into(), Default::default()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn can_export_messages_to_json() {
        let messages = vec![
            Message {
                rowid: 1,
                text: Some("Hello".to_string()),
                is_from_me: false,
                date: Utc.with_ymd_and_hms(2024, 1, 1, 12, 0, 0).unwrap().timestamp(),
                ..Message::blank()
            },
            Message {
                rowid: 2,
                text: Some("Hi".to_string()),
                is_from_me: true,
                date: Utc.with_ymd_and_hms(2024, 1, 1, 12, 1, 0).unwrap().timestamp(),
                ..Message::blank()
            },
        ];

        let json = export_messages(&messages).unwrap();
        assert!(json.contains("Hello"));
        assert!(json.contains("Hi"));
        assert!(json.contains("Me"));
        assert!(json.contains("Other"));
    }
} 