use local_apt::cli::{CommandName, OutputFormat};
use serde::Serialize;
use std::io::{self, Write};

pub const SCHEMA_VERSION: u8 = 1;

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Outcome {
    Success,
    PartialSuccess,
    Failure,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct Summary {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub downloaded: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unchanged: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub failed: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deleted: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kept: Option<u64>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(tag = "event", rename_all = "snake_case")]
pub enum Event {
    Started {
        command: CommandName,
    },
    Downloaded {
        source: String,
        package: String,
        version: String,
        path: String,
    },
    Unchanged {
        source: String,
    },
    PackageFailed {
        source: String,
        error: String,
    },
    Deleted {
        package: String,
        version: String,
        path: String,
    },
    Kept {
        package: String,
        version: String,
        path: String,
    },
    MetadataUpdated,
    Warning {
        message: String,
    },
    Fatal {
        error: String,
    },
    Finished {
        outcome: Outcome,
        summary: Summary,
    },
}

#[derive(Debug, Serialize)]
struct Report<'a> {
    schema_version: u8,
    command: CommandName,
    outcome: Outcome,
    summary: Summary,
    events: &'a [Event],
}

#[derive(Serialize)]
struct EventRecord<'a> {
    schema_version: u8,
    #[serde(flatten)]
    event: &'a Event,
}

pub struct Reporter<W: Write> {
    format: Option<OutputFormat>,
    writer: W,
    command: CommandName,
    events: Vec<Event>,
    pipe_closed: bool,
}

impl<W: Write> Reporter<W> {
    pub fn new(format: Option<OutputFormat>, writer: W, command: CommandName) -> Self {
        Self {
            format,
            writer,
            command,
            events: Vec::new(),
            pipe_closed: false,
        }
    }

    pub fn emit(&mut self, event: Event) -> Result<(), Error> {
        if matches!(self.format, Some(OutputFormat::Ndjson)) {
            self.write_json_line(&event)?;
        }
        self.events.push(event);
        Ok(())
    }

    pub fn finish(&mut self, outcome: Outcome, summary: Summary) -> Result<(), Error> {
        self.emit(Event::Finished {
            outcome: outcome.clone(),
            summary: summary.clone(),
        })?;

        if matches!(self.format, Some(OutputFormat::Json)) {
            let report = Report {
                schema_version: SCHEMA_VERSION,
                command: self.command.clone(),
                outcome,
                summary,
                events: &self.events,
            };
            let mut bytes = serde_json::to_vec(&report)?;
            bytes.push(b'\n');
            self.write_all(&bytes)?;
        }

        if !self.pipe_closed {
            self.writer.flush().map_err(Error::Write)?;
        }
        Ok(())
    }

    fn write_json_line(&mut self, event: &Event) -> Result<(), Error> {
        let mut bytes = serde_json::to_vec(&EventRecord {
            schema_version: SCHEMA_VERSION,
            event,
        })?;
        bytes.push(b'\n');
        self.write_all(&bytes)
    }

    fn write_all(&mut self, bytes: &[u8]) -> Result<(), Error> {
        if self.pipe_closed {
            return Ok(());
        }
        match self.writer.write_all(bytes) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == io::ErrorKind::BrokenPipe => {
                self.pipe_closed = true;
                Ok(())
            }
            Err(e) => Err(Error::Write(e)),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("failed to serialize output: {0}")]
    Serialize(#[from] serde_json::Error),
    #[error("failed to write output: {0}")]
    Write(io::Error),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_is_one_versioned_document() {
        let mut output = Vec::new();
        let mut reporter =
            Reporter::new(Some(OutputFormat::Json), &mut output, CommandName::Update);
        reporter
            .emit(Event::Started {
                command: CommandName::Update,
            })
            .unwrap();
        reporter
            .finish(Outcome::Success, Summary::default())
            .unwrap();

        let value: serde_json::Value = serde_json::from_slice(&output).unwrap();
        assert_eq!(value["schema_version"], SCHEMA_VERSION);
        assert_eq!(value["command"], "update");
        assert_eq!(value["outcome"], "success");
        assert_eq!(value["events"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn ndjson_contains_one_event_per_line() {
        let mut output = Vec::new();
        let mut reporter = Reporter::new(
            Some(OutputFormat::Ndjson),
            &mut output,
            CommandName::Cleanup,
        );
        reporter
            .emit(Event::Started {
                command: CommandName::Cleanup,
            })
            .unwrap();
        reporter
            .finish(Outcome::Success, Summary::default())
            .unwrap();

        let lines: Vec<_> = output.split(|byte| *byte == b'\n').collect();
        assert_eq!(lines.len(), 3);
        assert!(lines[2].is_empty());
        for line in &lines[..2] {
            let value = serde_json::from_slice::<serde_json::Value>(line).unwrap();
            assert_eq!(value["schema_version"], SCHEMA_VERSION);
        }
    }

    struct ClosedPipe;

    impl Write for ClosedPipe {
        fn write(&mut self, _buf: &[u8]) -> io::Result<usize> {
            Err(io::Error::from(io::ErrorKind::BrokenPipe))
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn broken_pipe_does_not_abort_reporting() {
        let mut reporter =
            Reporter::new(Some(OutputFormat::Ndjson), ClosedPipe, CommandName::Update);
        reporter
            .emit(Event::Started {
                command: CommandName::Update,
            })
            .unwrap();
        reporter
            .finish(Outcome::Success, Summary::default())
            .unwrap();
    }
}
