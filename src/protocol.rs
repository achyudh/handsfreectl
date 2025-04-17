use crate::cli::CliOutputMode;
use serde::{Deserialize, Serialize};

/// Commands that can be sent to the daemon
#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(tag = "command", rename_all = "lowercase")]
pub enum DaemonCommand {
    /// Start transcription with the specified output mode
    Start { output_mode: CliOutputMode },
    /// Stop transcription
    Stop,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_daemon_command_serialization() {
        let start_cmd = DaemonCommand::Start {
            output_mode: CliOutputMode::Clipboard,
        };
        let json = serde_json::to_string(&start_cmd).unwrap();
        assert_eq!(json, r#"{"command":"start","output_mode":"clipboard"}"#);

        let stop_cmd = DaemonCommand::Stop;
        let json = serde_json::to_string(&stop_cmd).unwrap();
        assert_eq!(json, r#"{"command":"stop"}"#);
    }

    #[test]
    fn test_daemon_command_deserialization() {
        // Test Start command deserialization
        let json = r#"{"command":"start","output_mode":"clipboard"}"#;
        let cmd: DaemonCommand = serde_json::from_str(json).unwrap();
        assert_eq!(
            cmd,
            DaemonCommand::Start {
                output_mode: CliOutputMode::Clipboard
            }
        );

        // Test Stop command deserialization
        let json = r#"{"command":"stop"}"#;
        let cmd: DaemonCommand = serde_json::from_str(json).unwrap();
        assert_eq!(cmd, DaemonCommand::Stop);
    }
}
