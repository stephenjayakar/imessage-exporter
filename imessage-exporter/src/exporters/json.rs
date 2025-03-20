/*!
 JSON export functionality.
*/

use std::{fs::File, io::BufWriter, collections::HashMap};
use serde::Serialize;
use crate::app::{error::RuntimeError, runtime::Config};
use imessage_database::{
    tables::{messages::Message, table::Table},
    error::table::TableError,
    message_types::variants::Variant,
};
use super::exporter::Exporter;

#[derive(Serialize)]
struct MessageJson {
    id: String,
    text: Option<String>,
    sender: String,
    timestamp: i64,
    #[serde(rename = "type")]
    message_type: String,
}

#[derive(Serialize)]
struct ConversationJson {
    participants: Vec<String>,
    messages: Vec<MessageJson>,
}

pub struct JSON<'a> {
    /// The file we are writing to
    writer: BufWriter<File>,
    /// The config for this export
    config: &'a Config,
}

impl<'a> Exporter<'a> for JSON<'a> {
    fn new(config: &'a Config) -> Result<Self, RuntimeError> {
        let path = config.options.export_path.join("messages.json");
        let file = File::create(&path).map_err(|e| RuntimeError::CreateError(e, path))?;
        let writer = BufWriter::new(file);
        Ok(JSON { writer, config })
    }

    fn iter_messages(&mut self) -> Result<(), RuntimeError> {
        let mut statement = Message::stream_rows(&self.config.db, &self.config.options.query_context)
            .map_err(RuntimeError::DatabaseError)?;

        let messages = statement
            .query_map([], |row| Message::from_row(row))
            .map_err(|e| RuntimeError::DatabaseError(TableError::Messages(e)))?;

        // Group messages by conversation
        let mut conversations: HashMap<String, Vec<MessageJson>> = HashMap::new();

        for msg in messages {
            let msg = msg.map_err(|e| RuntimeError::DatabaseError(TableError::Messages(e)))?;
            let mut msg = Message::extract(Ok(Ok(msg))).map_err(RuntimeError::DatabaseError)?;
            
            // Generate text content
            let _ = msg.generate_text(&self.config.db);

            let message_type = match msg.variant() {
                Variant::Tapback(..) => "reaction".to_string(),
                _ => "message".to_string(),
            };

            let json_msg = MessageJson {
                id: msg.guid.clone(),
                text: msg.text.clone(),
                sender: self.config.who(msg.handle_id, msg.is_from_me(), &msg.destination_caller_id).to_string(),
                timestamp: msg.date,
                message_type,
            };

            // Use chat_id as the conversation key
            let chat_id = msg.chat_id.map(|id| id.to_string()).unwrap_or_else(|| "unknown".to_string());
            conversations.entry(chat_id)
                .or_insert_with(Vec::new)
                .push(json_msg);
        }

        // Convert conversations to final format and sort messages by timestamp
        let mut final_conversations: Vec<ConversationJson> = conversations
            .into_iter()
            .map(|(_, mut messages)| {
                // Sort messages by timestamp
                messages.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));
                
                // Get unique participants from messages
                let mut participants: Vec<String> = messages.iter()
                    .map(|m| m.sender.clone())
                    .collect::<std::collections::HashSet<_>>()
                    .into_iter()
                    .collect();
                participants.sort();

                ConversationJson {
                    participants,
                    messages,
                }
            })
            .collect();

        // Sort conversations by most recent message
        final_conversations.sort_by(|a, b| {
            let a_latest = a.messages.last().map(|m| m.timestamp).unwrap_or(0);
            let b_latest = b.messages.last().map(|m| m.timestamp).unwrap_or(0);
            b_latest.cmp(&a_latest)
        });

        serde_json::to_writer_pretty(&mut self.writer, &final_conversations)
            .map_err(|e| RuntimeError::CreateError(std::io::Error::new(std::io::ErrorKind::Other, e), self.config.options.export_path.clone()))?;

        Ok(())
    }

    fn get_or_create_file(&mut self, _message: &Message) -> Result<&mut BufWriter<File>, RuntimeError> {
        Ok(&mut self.writer)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::export_type::ExportType;
    use crate::app::options::Options;
    use std::fs;
    use std::io::Read;

    #[test]
    fn can_export_messages_to_json() {
        let options = Options::fake_options(ExportType::Json);
        let config = Config::fake_app(options);
        let mut exporter = JSON::new(&config).unwrap();

        exporter.iter_messages().unwrap();

        let mut file = File::open("messages.json").unwrap();
        let mut contents = String::new();
        file.read_to_string(&mut contents).unwrap();

        // Note: We can't test exact contents since timestamps will vary
        assert!(contents.contains("\"participants\""));
        assert!(contents.contains("\"messages\""));
        assert!(contents.contains("\"id\""));
        assert!(contents.contains("\"text\""));
        assert!(contents.contains("\"sender\""));
        assert!(contents.contains("\"timestamp\""));
        assert!(contents.contains("\"type\""));

        // Cleanup
        fs::remove_file("messages.json").unwrap();
    }
} 