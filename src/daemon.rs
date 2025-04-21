use crate::protocol::DaemonCommand;
use std::env;
use std::path::{Path, PathBuf};
use tokio::io::AsyncWriteExt;
use tokio::net::UnixStream;

/// Get the path to the daemon's Unix domain socket
pub fn get_socket_path() -> Result<PathBuf, String> {
    match env::var("XDG_RUNTIME_DIR") {
        Ok(runtime_dir) => {
            let socket_path = PathBuf::from(runtime_dir).join("handsfree.sock");
            Ok(socket_path)
        }
        Err(e) => Err(format!("$XDG_RUNTIME_DIR not set or invalid: {}", e)),
    }
}

/// Connect to the daemon's Unix domain socket
pub async fn connect_to_daemon(socket_path: &Path) -> Result<UnixStream, String> {
    match UnixStream::connect(socket_path).await {
        Ok(stream) => {
            println!("Successfully connected to daemon at {:?}", socket_path);
            Ok(stream)
        }
        Err(e) => Err(format!(
            "Failed to connect to daemon socket at {:?}: {}. Is the daemon running?",
            socket_path, e
        )),
    }
}

/// Send a command to the daemon
pub async fn send_command(stream: &mut UnixStream, command: &DaemonCommand) -> Result<(), String> {
    let command_json = serde_json::to_string(command)
        .map_err(|e| format!("Failed to serialize command: {}", e))?;
    let command_json_with_newline = format!("{}\n", command_json);
    println!("Sending: {}", command_json_with_newline.trim()); // Trim newline for cleaner log

    stream
        .write_all(command_json_with_newline.as_bytes())
        .await
        .map_err(|e| format!("Failed to write command to socket: {}", e))?;

    stream
        .flush()
        .await
        .map_err(|e| format!("Failed to flush socket: {}", e))?;

    stream
        .shutdown()
        .await
        .map_err(|e| format!("Failed to shutdown socket write half: {}", e))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::CliOutputMode;
    use crate::protocol::DaemonCommand;
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    use std::time::Duration;
    use tempfile;
    use tokio::io::AsyncReadExt;
    use tokio::net::UnixListener;

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

        let expected_path = temp_dir.path().join("handsfree.sock");
        match get_socket_path() {
            Ok(path) => assert_eq!(path, expected_path),
            Err(e) => panic!("get_socket_path failed unexpectedly: {}", e),
        }

        // SAFETY: We're in a test and managing the env var lifecycle
        unsafe {
            env::remove_var("XDG_RUNTIME_DIR"); // Clean up env var
        }
    }

    #[test]
    fn test_get_socket_path_no_xdg_runtime_dir() {
        // SAFETY: We're in a test and managing the env var lifecycle
        unsafe {
            // Ensure XDG_RUNTIME_DIR is not set
            env::remove_var("XDG_RUNTIME_DIR");
        }
        // Try to get socket path - should fail when XDG_RUNTIME_DIR is not set
        match get_socket_path() {
            Ok(_) => panic!("get_socket_path unexpectedly succeeded without XDG_RUNTIME_DIR"),
            Err(e) => assert!(e.contains("XDG_RUNTIME_DIR")), // Error message should mention XDG_RUNTIME_DIR
        }
    }

    #[tokio::test]
    async fn test_send_command() {
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
            String::from_utf8_lossy(&buf[..n]).to_string()
        });

        // Connect and send a command
        let mut stream = UnixStream::connect(&socket_path).await.unwrap();
        let command = DaemonCommand::Start {
            output_mode: CliOutputMode::Clipboard,
        };
        send_command(&mut stream, &command).await.unwrap();

        // Get the received data and verify it
        let received = handle.await.unwrap();
        let expected = format!("{}\n", serde_json::to_string(&command).unwrap());
        assert_eq!(received, expected);
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
}
