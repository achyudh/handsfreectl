use crate::protocol::{DaemonCommand, DaemonResponse};
use log::{debug, warn};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;
use tokio::time::timeout;
use users::get_current_uid;

const READ_TIMEOUT_SECS: u64 = 5; // Timeout for waiting for response

/// Get the path to the daemon's Unix domain socket, matching daemon defaults.
pub fn get_socket_path() -> Result<PathBuf, String> {
    if let Ok(runtime_dir_str) = env::var("XDG_RUNTIME_DIR") {
        let runtime_dir = PathBuf::from(runtime_dir_str);
        let socket_dir = runtime_dir.join("handsfree");

        // Attempt to create the directory
        match fs::create_dir_all(&socket_dir) {
            Ok(_) => {
                let socket_path = socket_dir.join("daemon.sock");
                debug!("Using socket path: {:?}", socket_path);
                return Ok(socket_path);
            }
            Err(e) => {
                warn!(
                    "Warning: Could not create directory in XDG_RUNTIME_DIR ({}): {}. \
                     Falling back to /tmp.",
                    socket_dir.display(),
                    e
                );
                // Fall through to /tmp fallback
            }
        }
    } else {
        warn!("Warning: XDG_RUNTIME_DIR not set. Falling back to /tmp.");
        // Fall through to /tmp fallback
    }

    // Fallback logic: use uid-specific socket in /tmp
    let uid = get_current_uid();
    let socket_path = PathBuf::from(format!("/tmp/handsfree-{}.sock", uid));
    debug!("Using fallback socket path: {:?}", socket_path);
    Ok(socket_path)
}

/// Connect to the daemon's Unix domain socket
pub async fn connect_to_daemon(socket_path: &Path) -> Result<UnixStream, std::io::Error> {
    match UnixStream::connect(socket_path).await {
        Ok(stream) => {
            debug!("Successfully connected to daemon at {:?}", socket_path);
            Ok(stream)
        }
        Err(e) => Err(e),
    }
}

/// Reads and deserializes a JSON response line from the daemon stream.
pub async fn receive_response(stream: &mut UnixStream) -> Result<DaemonResponse, String> {
    let mut reader = BufReader::new(stream);
    let mut response_json = String::new();

    // Add a timeout for reading the response
    match timeout(
        std::time::Duration::from_secs(READ_TIMEOUT_SECS),
        reader.read_line(&mut response_json),
    )
    .await
    {
        Ok(Ok(0)) | Err(_) => {
            // Ok(Ok(0)) -> EOF, Err(_) -> TimeoutError
            if response_json.is_empty() {
                Err(format!(
                    "Connection closed by daemon or timeout after {} seconds while waiting for response.",
                    READ_TIMEOUT_SECS
                ))
            } else {
                // Timeout occurred but maybe we read something? Less likely with read_line
                Err(format!(
                    "Timeout after {} seconds reading response line.",
                    READ_TIMEOUT_SECS
                ))
            }
        }
        Ok(Ok(_)) => {
            // Successfully read a line
            let trimmed_response = response_json.trim_end_matches('\n');
            if trimmed_response.is_empty() {
                Err("Received empty response line from daemon.".to_string())
            } else {
                serde_json::from_str::<DaemonResponse>(trimmed_response).map_err(|e| {
                    format!(
                        "Failed to deserialize daemon response '{}': {}",
                        trimmed_response, e
                    )
                })
            }
        }
        Ok(Err(e)) => {
            // IO error from read_line
            Err(format!("Failed to read response from daemon: {}", e))
        }
    }
}

/// Send a command to the daemon and read its response
pub async fn send_command(
    stream: &mut UnixStream,
    command: &DaemonCommand,
) -> Result<DaemonResponse, String> {
    let command_json = serde_json::to_string(command)
        .map_err(|e| format!("Failed to serialize command: {}", e))?;
    let command_json_with_newline = format!("{}\n", command_json);
    debug!("Sending: {}", command_json_with_newline.trim()); // Trim newline for cleaner log

    stream
        .write_all(command_json_with_newline.as_bytes())
        .await
        .map_err(|e| format!("Failed to write command to socket: {}", e))?;

    stream
        .flush()
        .await
        .map_err(|e| format!("Failed to flush socket: {}", e))?;

    debug!("Waiting for response...");
    // Don't shutdown, we need to read the response
    receive_response(stream).await
}

/// Serialize and send a command to the daemon without waiting for a response.
/// Useful for commands like Subscribe where the response is a stream.
pub async fn send_command_only(
    stream: &mut UnixStream,
    command: &DaemonCommand,
) -> Result<(), String> {
    let command_json = serde_json::to_string(command)
        .map_err(|e| format!("Failed to serialize command: {}", e))?;
    let command_json_with_newline = format!("{}\n", command_json);
    debug!("Sending only: {}", command_json_with_newline.trim());

    stream
        .write_all(command_json_with_newline.as_bytes())
        .await
        .map_err(|e| format!("Failed to write command to socket: {}", e))?;

    stream
        .flush()
        .await
        .map_err(|e| format!("Failed to flush socket: {}", e))?;

    Ok(())
}

/// A stream of responses from the daemon.
/// Wraps the UnixStream and handles reading lines and deserializing JSON.
pub struct ResponseStream {
    reader: BufReader<UnixStream>,
}

impl ResponseStream {
    pub fn new(stream: UnixStream) -> Self {
        Self {
            reader: BufReader::new(stream),
        }
    }

    /// Next response from the stream.
    /// Returns None on EOF.
    pub async fn next(&mut self) -> Option<Result<DaemonResponse, String>> {
        let mut line = String::new();
        loop {
            line.clear();
            match self.reader.read_line(&mut line).await {
                Ok(0) => return None, // EOF
                Ok(_) => {
                    let trimmed = line.trim_end_matches('\n');
                    if trimmed.trim().is_empty() {
                        continue;
                    }
                    return Some(
                        serde_json::from_str::<DaemonResponse>(trimmed).map_err(|e| {
                            format!("Failed to deserialize: {} (Line: {})", e, trimmed)
                        }),
                    );
                }
                Err(e) => return Some(Err(format!("IO Error: {}", e))),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::CliOutputMode;
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    use std::time::Duration;
    use tempfile;
    use tokio::io::AsyncReadExt;
    use tokio::net::UnixListener;

    // Helper struct for tests to manage environment variables temporarily
    struct EnvVarGuard {
        key: String,
        original_value: Option<String>,
    }

    impl EnvVarGuard {
        fn new(key: &str) -> Self {
            let key = key.to_string();
            let original_value = env::var(&key).ok();
            // SAFETY: We're in a test and managing the env var lifecycle
            unsafe {
                env::remove_var(&key);
            } // Ensure it's removed for the test
            EnvVarGuard {
                key,
                original_value,
            }
        }
    }

    impl Drop for EnvVarGuard {
        fn drop(&mut self) {
            // SAFETY: We're in a test and managing the env var lifecycle
            unsafe {
                // Restore original value when guard goes out of scope
                match &self.original_value {
                    Some(val) => env::set_var(&self.key, val),
                    None => env::remove_var(&self.key),
                }
            }
        }
    }

    #[test]
    fn test_get_socket_path_success() {
        // Mock XDG_RUNTIME_DIR for this test
        let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
        let temp_path = temp_dir.path();

        // Set correct permissions (0o700) on the temporary directory
        fs::set_permissions(temp_path, fs::Permissions::from_mode(0o700))
            .expect("Failed to set directory permissions");

        let temp_path_str = temp_path.to_str().expect("Failed to get path string");
        // SAFETY: We're in a test and managing the env var lifecycle
        unsafe {
            env::set_var("XDG_RUNTIME_DIR", temp_path_str);
        }

        let expected_path = temp_path.join("handsfree").join("daemon.sock");
        match get_socket_path() {
            Ok(path) => assert_eq!(path, expected_path),
            Err(e) => panic!("get_socket_path failed unexpectedly: {}", e),
        }

        // Verify the directory was created
        let handsfree_dir = temp_path.join("handsfree");
        assert!(handsfree_dir.exists());
        assert!(handsfree_dir.is_dir());

        // SAFETY: We're in a test and managing the env var lifecycle
        unsafe {
            env::remove_var("XDG_RUNTIME_DIR"); // Clean up env var
        }
    }

    #[test]
    fn test_get_socket_path_no_xdg_runtime_dir() {
        // Temporarily remove the var for this test's scope if set externally
        let _env_guard = EnvVarGuard::new("XDG_RUNTIME_DIR");

        // Get the expected UID for comparison
        let uid = get_current_uid();
        let expected_path = PathBuf::from(format!("/tmp/handsfree-{}.sock", uid));

        match get_socket_path() {
            Ok(path) => assert_eq!(path, expected_path),
            Err(e) => panic!("get_socket_path failed unexpectedly: {}", e),
        }
    }

    #[test]
    fn test_get_socket_path_xdg_create_fails() {
        let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
        let temp_path = temp_dir.path();

        // Create a file where the directory should be to force creation failure
        let handsfree_path = temp_path.join("handsfree");
        fs::write(&handsfree_path, "block directory creation")
            .expect("Failed to create blocking file");

        let temp_path_str = temp_path.to_str().expect("Failed to get path string");
        // SAFETY: We're in a test and managing the env var lifecycle
        unsafe {
            env::set_var("XDG_RUNTIME_DIR", temp_path_str);
        }

        // Should fall back to /tmp/handsfree-uid.sock
        let uid = get_current_uid();
        let expected_path = PathBuf::from(format!("/tmp/handsfree-{}.sock", uid));

        match get_socket_path() {
            Ok(path) => assert_eq!(path, expected_path),
            Err(e) => panic!("get_socket_path failed unexpectedly: {}", e),
        }

        // SAFETY: We're in a test and managing the env var lifecycle
        unsafe {
            env::remove_var("XDG_RUNTIME_DIR");
        }
    }

    // Test successful command sending and response parsing
    #[tokio::test]
    async fn test_send_command_with_response() {
        // Create a temporary socket path
        let dir = tempfile::tempdir().unwrap();
        let socket_path = dir.path().join("test.sock");

        // Create a listener
        let listener = UnixListener::bind(&socket_path).unwrap();

        // Spawn a task to handle the connection
        let handle = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut buf = [0u8; 1024];
            let n = socket.read(&mut buf).await.unwrap();
            let received = String::from_utf8_lossy(&buf[..n]);

            // Send back an Ack response
            let response = r#"{"response_type":"ack"}"#;
            socket
                .write_all(format!("{}\n", response).as_bytes())
                .await
                .unwrap();
            socket.flush().await.unwrap();
            received.to_string()
        });

        // Connect and send a command
        let mut stream = UnixStream::connect(&socket_path).await.unwrap();
        let command = DaemonCommand::Start {
            output_mode: CliOutputMode::Clipboard,
        };

        // Send command and get response
        let response = send_command(&mut stream, &command).await.unwrap();

        // Verify sent command
        let received = handle.await.unwrap();
        let expected = format!("{}\n", serde_json::to_string(&command).unwrap());
        assert_eq!(received, expected);

        // Verify received response
        assert!(matches!(response, DaemonResponse::Ack));
    }

    // Test response timeout
    #[tokio::test]
    async fn test_receive_response_timeout() {
        // Create a temporary socket path
        let dir = tempfile::tempdir().unwrap();
        let socket_path = dir.path().join("test.sock");

        // Create a listener that accepts but never responds
        let listener = UnixListener::bind(&socket_path).unwrap();
        tokio::spawn(async move {
            let (_socket, _) = listener.accept().await.unwrap();
            // Don't send any response, just wait
            tokio::time::sleep(Duration::from_secs(10)).await;
        });

        // Connect and try to receive
        let mut stream = UnixStream::connect(&socket_path).await.unwrap();
        let result = receive_response(&mut stream).await;

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("timeout"));
    }

    // Test invalid response JSON
    #[tokio::test]
    async fn test_receive_invalid_response() {
        // Create a temporary socket path
        let dir = tempfile::tempdir().unwrap();
        let socket_path = dir.path().join("test.sock");

        // Create a listener that sends invalid JSON
        let listener = UnixListener::bind(&socket_path).unwrap();
        tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            socket.write_all(b"{invalid_json}\n").await.unwrap();
            socket.flush().await.unwrap();
        });

        // Connect and try to receive
        let mut stream = UnixStream::connect(&socket_path).await.unwrap();
        let result = receive_response(&mut stream).await;

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Failed to deserialize"));
    }

    #[tokio::test]
    async fn test_daemon_error_response() {
        // Create a temporary socket path
        let dir = tempfile::tempdir().unwrap();
        let socket_path = dir.path().join("test.sock");

        // Create a listener that sends an error response
        let listener = UnixListener::bind(&socket_path).unwrap();
        tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let error_response = r#"{"response_type":"error","message":"Invalid command"}"#;
            socket
                .write_all(format!("{}\n", error_response).as_bytes())
                .await
                .unwrap();
            socket.flush().await.unwrap();
        });

        // Connect and try to receive
        let mut stream = UnixStream::connect(&socket_path).await.unwrap();
        let response = receive_response(&mut stream).await.unwrap();

        match response {
            DaemonResponse::Error { message } => {
                assert_eq!(message, "Invalid command");
            }
            other => panic!("Expected Error response, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_connection_timeout() {
        // Try to connect to a non-existent socket
        let dir = tempfile::tempdir().unwrap();
        let socket_path = dir.path().join("nonexistent.sock");

        // Set a timeout using tokio::time::timeout
        let result =
            tokio::time::timeout(Duration::from_millis(100), connect_to_daemon(&socket_path)).await;

        // Should either timeout or fail to connect
        assert!(result.is_err() || result.unwrap().is_err());
    }

    #[test]
    fn test_error_kind_for_nonexistent_socket() {
        // This test verifies that the error kind for a nonexistent socket file is NotFound
        let runtime = tokio::runtime::Runtime::new().unwrap();
        let dir = tempfile::tempdir().unwrap();
        let socket_path = dir.path().join("nonexistent.sock");

        let result = runtime.block_on(connect_to_daemon(&socket_path));
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.kind(), std::io::ErrorKind::NotFound);
    }
}
