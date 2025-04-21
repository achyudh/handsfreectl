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
    /// Get daemon status
    Status,
    /// Tell daemon to shut down gracefully
    Shutdown,
}

/// Status information returned by the daemon
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct DaemonStatus {
    pub state: String,
    pub last_error: Option<String>,
}

/// All possible responses from the daemon
#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(tag = "response_type", rename_all = "snake_case")]
pub enum DaemonResponse {
    /// Simple acknowledgment of a command
    Ack,
    /// Status information response
    Status(DaemonStatus),
    /// Error response with message
    Error { message: String },
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

        let status_cmd = DaemonCommand::Status;
        let json = serde_json::to_string(&status_cmd).unwrap();
        assert_eq!(json, r#"{"command":"status"}"#);

        let shutdown_cmd = DaemonCommand::Shutdown;
        let json = serde_json::to_string(&shutdown_cmd).unwrap();
        assert_eq!(json, r#"{"command":"shutdown"}"#);
    }

    #[test]
    fn test_daemon_command_deserialization() {
        // Test Start command
        let json = r#"{"command":"start","output_mode":"clipboard"}"#;
        let cmd: DaemonCommand = serde_json::from_str(json).unwrap();
        assert_eq!(
            cmd,
            DaemonCommand::Start {
                output_mode: CliOutputMode::Clipboard
            }
        );

        // Test Stop command
        let json = r#"{"command":"stop"}"#;
        let cmd: DaemonCommand = serde_json::from_str(json).unwrap();
        assert_eq!(cmd, DaemonCommand::Stop);

        // Test Status command
        let json = r#"{"command":"status"}"#;
        let cmd: DaemonCommand = serde_json::from_str(json).unwrap();
        assert_eq!(cmd, DaemonCommand::Status);

        // Test Shutdown command
        let json = r#"{"command":"shutdown"}"#;
        let cmd: DaemonCommand = serde_json::from_str(json).unwrap();
        assert_eq!(cmd, DaemonCommand::Shutdown);
    }

    #[test]
    fn test_daemon_response_deserialization() {
        // Test Ack
        let json_ack = r#"{"response_type":"ack"}"#;
        let resp_ack: DaemonResponse = serde_json::from_str(json_ack).unwrap();
        assert_eq!(resp_ack, DaemonResponse::Ack);

        // Test Status (with error)
        let json_status =
            r#"{"response_type":"status","state":"error","last_error":"Model failed"}"#;
        let resp_status: DaemonResponse = serde_json::from_str(json_status).unwrap();
        assert_eq!(
            resp_status,
            DaemonResponse::Status(DaemonStatus {
                state: "error".to_string(),
                last_error: Some("Model failed".to_string())
            })
        );

        // Test Status (no error)
        let json_status_ok = r#"{"response_type":"status","state":"idle","last_error":null}"#;
        let resp_status_ok: DaemonResponse = serde_json::from_str(json_status_ok).unwrap();
        assert_eq!(
            resp_status_ok,
            DaemonResponse::Status(DaemonStatus {
                state: "idle".to_string(),
                last_error: None
            })
        );

        // Test Error
        let json_error = r#"{"response_type":"error","message":"Bad command"}"#;
        let resp_error: DaemonResponse = serde_json::from_str(json_error).unwrap();
        assert_eq!(
            resp_error,
            DaemonResponse::Error {
                message: "Bad command".to_string()
            }
        );
    }

    #[test]
    fn test_daemon_response_serialization() {
        // Test Ack serialization
        let resp_ack = DaemonResponse::Ack;
        let json = serde_json::to_string(&resp_ack).unwrap();
        assert_eq!(json, r#"{"response_type":"ack"}"#);

        // Test Status serialization (with error)
        let resp_status = DaemonResponse::Status(DaemonStatus {
            state: "error".to_string(),
            last_error: Some("Model failed".to_string()),
        });
        let json = serde_json::to_string(&resp_status).unwrap();
        assert_eq!(
            json,
            r#"{"response_type":"status","state":"error","last_error":"Model failed"}"#
        );

        // Test Status serialization (no error)
        let resp_status_ok = DaemonResponse::Status(DaemonStatus {
            state: "idle".to_string(),
            last_error: None,
        });
        let json = serde_json::to_string(&resp_status_ok).unwrap();
        assert_eq!(
            json,
            r#"{"response_type":"status","state":"idle","last_error":null}"#
        );

        // Test Error serialization
        let resp_error = DaemonResponse::Error {
            message: "Bad command".to_string(),
        };
        let json = serde_json::to_string(&resp_error).unwrap();
        assert_eq!(json, r#"{"response_type":"error","message":"Bad command"}"#);
    }
}
