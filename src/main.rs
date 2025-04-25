use handsfreectl::cli::{Cli, Commands};
use handsfreectl::daemon::{connect_to_daemon, get_socket_path, send_command};
use handsfreectl::protocol::{DaemonCommand, DaemonResponse};
use log::{debug, error, warn};
use std::io::ErrorKind;

#[tokio::main]
async fn main() {
    env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or("handsfreectl=warn"),
    )
    .init();

    let cli = Cli::parse();

    let socket_path = match get_socket_path() {
        Ok(path) => path,
        Err(e) => {
            error!("Error determining socket path: {}", e);
            std::process::exit(1);
        }
    };

    if matches!(cli.command, Commands::Status) {
        match connect_to_daemon(&socket_path).await {
            Ok(mut stream) => {
                debug!("Sending command: {:?}", DaemonCommand::Status);

                match send_command(&mut stream, &DaemonCommand::Status).await {
                    Ok(response) => match response {
                        DaemonResponse::Status { status } => {
                            println!("{}", status.state);
                            if let Some(err) = status.last_error {
                                println!("{}", err);
                            }
                        }
                        DaemonResponse::Error { message } => {
                            error!("Daemon Error: {}", message);
                            std::process::exit(1);
                        }
                        DaemonResponse::Ack => {
                            warn!("Received unexpected Ack for Status command");
                        }
                    },
                    Err(e) => {
                        error!("Communication Error: {}", e);
                        std::process::exit(1);
                    }
                }
            }
            Err(e) => {
                if matches!(e.kind(), ErrorKind::NotFound | ErrorKind::ConnectionRefused) {
                    println!("Inactive");
                    std::process::exit(0);
                } else {
                    error!(
                        "Connection Error: Failed to connect to daemon socket at {:?}: {}. Is the daemon running?",
                        socket_path, e
                    );
                    std::process::exit(1);
                }
            }
        }
    } else {
        match connect_to_daemon(&socket_path).await {
            Ok(mut stream) => {
                // Convert CLI command to daemon command
                let daemon_command = match &cli.command {
                    Commands::Start { output } => DaemonCommand::Start {
                        output_mode: output.clone(),
                    },
                    Commands::Stop => DaemonCommand::Stop,
                    Commands::Shutdown => DaemonCommand::Shutdown,
                    Commands::Status => unreachable!(), // Handled in the other branch
                };

                debug!("Sending command: {:?}", daemon_command);

                match send_command(&mut stream, &daemon_command).await {
                    Ok(response) => match response {
                        DaemonResponse::Ack => {
                            println!("OK");
                        }
                        DaemonResponse::Status { .. } => {
                            warn!("Received unexpected Status response for non-status command");
                            println!("OK");
                        }
                        DaemonResponse::Error { message } => {
                            error!("Daemon Error: {}", message);
                            std::process::exit(1);
                        }
                    },
                    Err(e) => {
                        error!("Communication Error: {}", e);
                        std::process::exit(1);
                    }
                }
            }
            Err(e) => {
                error!(
                    "Connection Error: Failed to connect to daemon socket at {:?}: {}. Is the daemon running?",
                    socket_path, e
                );
                std::process::exit(1);
            }
        }
    }
}
