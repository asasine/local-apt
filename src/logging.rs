use local_apt::cli::RepoArgs;
use syslog_tracing::{Facility, Options, Syslog};
use thiserror::Error;
use tracing::warn;
use tracing_subscriber::{
    EnvFilter, Layer, filter::LevelFilter, layer::SubscriberExt, util::SubscriberInitExt,
};

pub fn init(args: &RepoArgs) -> Result<(), Error> {
    let stderr_filter = stderr_filter(args)?;
    let syslog = Syslog::new(c"local-apt", Options::LOG_PID, Facility::User);
    let syslog_unavailable = syslog.is_none();
    let syslog_layer = syslog.map(|writer| {
        tracing_subscriber::fmt::layer()
            .with_writer(writer)
            .with_ansi(false)
            .without_time()
            .with_level(false)
            .with_target(false)
            .with_filter(LevelFilter::INFO)
    });
    let stderr_layer = tracing_subscriber::fmt::layer()
        .with_writer(std::io::stderr)
        .with_filter(stderr_filter);

    tracing_subscriber::registry()
        .with(stderr_layer)
        .with(syslog_layer)
        .try_init()
        .map_err(|e| Error::Initialize(e.to_string()))?;

    if syslog_unavailable {
        warn!("syslog is unavailable; continuing with stderr only");
    }
    Ok(())
}

fn stderr_filter(args: &RepoArgs) -> Result<EnvFilter, Error> {
    let directive = if args.quiet {
        "error".to_owned()
    } else if args.verbose == 1 {
        "info".to_owned()
    } else if args.verbose == 2 {
        "debug".to_owned()
    } else if args.verbose >= 3 {
        "trace".to_owned()
    } else if let Ok(filter) = std::env::var("RUST_LOG") {
        filter
    } else {
        "warn".to_owned()
    };
    EnvFilter::try_new(&directive).map_err(|e| Error::InvalidFilter {
        directive,
        source: e,
    })
}

#[derive(Debug, Error)]
pub enum Error {
    #[error(r#"invalid log filter "{directive}": {source}"#)]
    InvalidFilter {
        directive: String,
        source: tracing_subscriber::filter::ParseError,
    },
    #[error("failed to initialize logging: {0}")]
    Initialize(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalid_filter_error_includes_directive_and_parse_error() {
        let directive = "[invalid".to_owned();
        let source = EnvFilter::try_new(&directive).unwrap_err();
        let parse_error = source.to_string();
        let error = Error::InvalidFilter { directive, source };

        assert_eq!(
            error.to_string(),
            format!(r#"invalid log filter "[invalid": {parse_error}"#)
        );
    }
}
