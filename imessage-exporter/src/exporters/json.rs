/*!
 JSON export functionality.
*/

use std::{fs::File, io::BufWriter};
use serde::Serialize;
use crate::app::{error::RuntimeError, runtime::Config};
use imessage_database::{
    tables::{messages::Message, table::Table},
    error::table::TableError,
};
use super::exporter::Exporter;

#[derive(Serialize)]
struct MessageJson {
    id: String,
    text: Option<String>,
    sender: String,
    timestamp: i64,
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
        let mut statement = Message::get(&self.config.db)
            .map_err(RuntimeError::DatabaseError)?;

        let messages = statement
            .query_map([], |row| Message::from_row(row))
            .map_err(|e| RuntimeError::DatabaseError(TableError::Messages(e)))?;

        let mut json_messages = Vec::new();

        for msg in messages {
            let msg = msg.map_err(|e| RuntimeError::DatabaseError(TableError::Messages(e)))?;
            let msg = Message::extract(Ok(Ok(msg))).map_err(RuntimeError::DatabaseError)?;

            let json_msg = MessageJson {
                id: msg.guid,
                text: msg.text,
                sender: if msg.is_from_me { "Me".to_string() } else { "Other".to_string() },
                timestamp: msg.date,
            };

            json_messages.push(json_msg);
        }

        serde_json::to_writer_pretty(&mut self.writer, &json_messages)
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
        assert!(contents.contains("\"id\""));
        assert!(contents.contains("\"text\""));
        assert!(contents.contains("\"sender\""));
        assert!(contents.contains("\"timestamp\""));

        // Cleanup
        fs::remove_file("messages.json").unwrap();
    }
} 