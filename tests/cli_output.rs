use std::{
    fs,
    process::{Command, Output},
};

struct TestCommand {
    temp: tempfile::TempDir,
    output_format: Option<&'static str>,
}

impl TestCommand {
    fn new() -> Self {
        let temp = tempfile::tempdir().unwrap();
        fs::write(temp.path().join("packages.toml"), "").unwrap();
        Self {
            temp,
            output_format: None,
        }
    }

    fn config(self, contents: &str) -> Self {
        fs::write(self.temp.path().join("packages.toml"), contents).unwrap();
        self
    }

    fn without_config(self) -> Self {
        fs::remove_file(self.temp.path().join("packages.toml")).unwrap();
        self
    }

    fn output(mut self, format: &'static str) -> Self {
        self.output_format = Some(format);
        self
    }

    fn run(self) -> Output {
        let mut command = Command::new(env!("CARGO_BIN_EXE_local-apt"));
        command
            .env("LOCAL_APT_CONFIG", self.temp.path().join("packages.toml"))
            .arg("update")
            .arg("--repository-directory")
            .arg(self.temp.path().join("repo"));
        if let Some(format) = self.output_format {
            command.arg(format!("--output={format}"));
        }
        command.output().unwrap()
    }
}

#[test]
fn successful_default_output_is_silent() {
    let output = TestCommand::new().run();

    assert!(output.status.success());
    assert!(output.stdout.is_empty());
}

#[test]
fn json_output_is_a_single_document() {
    let output = TestCommand::new().output("json").run();

    assert!(output.status.success());
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["schema_version"], 1);
    assert_eq!(value["command"], "update");
    assert_eq!(value["outcome"], "success");
}

#[test]
fn ndjson_output_has_one_document_per_line() {
    let output = TestCommand::new().output("ndjson").run();

    assert!(output.status.success());
    let lines = output
        .stdout
        .split(|byte| *byte == b'\n')
        .filter(|line| !line.is_empty());
    for line in lines {
        let value = serde_json::from_slice::<serde_json::Value>(line).unwrap();
        assert_eq!(value["schema_version"], 1);
    }
}

#[test]
fn fatal_json_output_remains_valid() {
    let output = TestCommand::new().without_config().output("json").run();

    assert!(!output.status.success());
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["outcome"], "failure");
    assert!(value["events"].as_array().unwrap().iter().any(|event| {
        event["event"] == "fatal"
            && event["error"]
                .as_str()
                .unwrap()
                .contains("failed to read configuration")
    }));
}

#[test]
fn configuration_errors_do_not_echo_secret_source_lines() {
    let output = TestCommand::new()
        .config("[[package]]\ntype = \"url\"\nurl = https://example.invalid/pkg.deb?token=secret\n")
        .output("json")
        .run();

    assert!(!output.status.success());
    assert!(!String::from_utf8_lossy(&output.stdout).contains("token=secret"));
    assert!(!String::from_utf8_lossy(&output.stderr).contains("token=secret"));
}
