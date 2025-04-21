use crate::protocol::DaemonCommand;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use tokio::io::AsyncWriteExt;
use tokio::net::UnixStream;
use users::get_current_uid;

/// Get the path to the daemon's Unix domain socket, matching daemon defaults.
pub fn get_socket_path() -> Result<PathBuf, String> {
    if let Ok(runtime_dir_str) = env::var("XDG_RUNTIME_DIR") {
        let runtime_dir = PathBuf::from(runtime_dir_str);
        let socket_dir = runtime_dir.join("handsfree");

        // Attempt to create the directory
        match fs::create_dir_all(&socket_dir) {
            Ok(_) => {
                let socket_path = socket_dir.join("daemon.sock");
                println!("Using socket path: {:?}", socket_path);
                return Ok(socket_path);
            }
            Err(e) => {
                eprintln!(
                    "Warning: Could not create directory in XDG_RUNTIME_DIR ({}): {}. \
                     Falling back to /tmp.",
                    socket_dir.display(),
                    e
                );
                // Fall through to /tmp fallback
            }
        }
    } else {
        eprintln!("Warning: XDG_RUNTIME_DIR not set. Falling back to /tmp.");
        // Fall through to /tmp fallback
    }

    // Fallback logic: use uid-specific socket in /tmp
    let uid = get_current_uid();
    let socket_path = PathBuf::from(format!("/tmp/handsfree-{}.sock", uid));
    println!("Using fallback socket path: {:?}", socket_path);
    Ok(socket_path)
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
