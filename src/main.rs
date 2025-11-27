use handsfreectl::cli::{Cli, Commands};
use handsfreectl::daemon::{
    ResponseStream, connect_to_daemon, get_socket_path, send_command, send_command_only,
};
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

    let mut stream = match connect_to_daemon(&socket_path).await {
        Ok(stream) => stream,
        Err(e) => {
            if let Commands::Status = cli.command {
                if matches!(e.kind(), ErrorKind::NotFound | ErrorKind::ConnectionRefused) {
                    println!("Inactive");
                    std::process::exit(0);
                }
            }

            error!(
                "Connection Error: Failed to connect to daemon socket at {:?}: {}. Is the daemon running?",
                socket_path, e
            );
            std::process::exit(1);
        }
    };

    match cli.command {
        Commands::Status => {
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
                    _ => {
                        warn!("Received unexpected response for Status command");
                    }
                },
                Err(e) => {
                    error!("Communication Error: {}", e);
                    std::process::exit(1);
                }
            }
        }
        Commands::Watch => {
            debug!("Sending command: {:?}", DaemonCommand::Subscribe);

            if let Err(e) = send_command_only(&mut stream, &DaemonCommand::Subscribe).await {
                error!("Failed to send subscribe command: {}", e);
                std::process::exit(1);
            }

            let mut response_stream = ResponseStream::new(stream);

            while let Some(result) = response_stream.next().await {
                match result {
                    Ok(response) => match response {
                        DaemonResponse::StateChange { status }
                        | DaemonResponse::Status { status } => {
                            println!("State changed: {}", status.state);
                            if let Some(err) = status.last_error {
                                println!("Error: {}", err);
                            }
                        }
                        DaemonResponse::Error { message } => {
                            error!("Daemon Error: {}", message);
                        }
                        _ => {}
                    },
                    Err(e) => {
                        warn!("{}", e);
                    }
                }
            }
            debug!("Stream closed");
        }
        _ => {
            let daemon_command = match &cli.command {
                Commands::Start { output } => DaemonCommand::Start {
                    output_mode: output.clone(),
                },
                Commands::Stop => DaemonCommand::Stop,
                Commands::Shutdown => DaemonCommand::Shutdown,
                Commands::Toggle { output } => DaemonCommand::Toggle {
                    output_mode: output.clone(),
                },
                _ => unreachable!(), // Handled in other branches
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
                    _ => {
                        warn!("Received unexpected response");
                    }
                },
                Err(e) => {
                    error!("Communication Error: {}", e);
                    std::process::exit(1);
                }
            }
        }
    }
}
