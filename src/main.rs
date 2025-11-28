use anyhow::{Context, Result, anyhow};
use handsfreectl::cli::{Cli, Commands};
use handsfreectl::daemon::{
    ResponseStream, connect_to_daemon, get_socket_path, send_command, send_command_only,
};
use handsfreectl::protocol::{DaemonCommand, DaemonResponse};
use log::{debug, error, warn};

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or("handsfreectl=warn"),
    )
    .init();

    let cli = Cli::parse();

    let socket_path = get_socket_path().context("Error determining socket path")?;

    let mut stream = match connect_to_daemon(&socket_path).await {
        Ok(stream) => stream,
        Err(e) => {
            if let Commands::Status = cli.command {
                // Check if it's a connection error (NotFound or ConnectionRefused)
                if let Some(io_err) = e.root_cause().downcast_ref::<std::io::Error>() {
                    if matches!(
                        io_err.kind(),
                        std::io::ErrorKind::NotFound | std::io::ErrorKind::ConnectionRefused
                    ) {
                        println!("Inactive");
                        return Ok(());
                    }
                }
            }

            return Err(e).with_context(|| {
                format!(
                    "Connection Error: Failed to connect to daemon socket at {:?}. Is the daemon running?",
                    socket_path
                )
            });
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
                        return Err(anyhow!("Daemon Error: {}", message));
                    }
                    _ => {
                        warn!("Received unexpected response for Status command");
                    }
                },
                Err(e) => {
                    return Err(e).context("Communication Error");
                }
            }
        }
        Commands::Watch => {
            debug!("Sending command: {:?}", DaemonCommand::Subscribe);

            send_command_only(&mut stream, &DaemonCommand::Subscribe)
                .await
                .context("Failed to send subscribe command")?;

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
                        return Err(anyhow!("Daemon Error: {}", message));
                    }
                    _ => {
                        warn!("Received unexpected response");
                    }
                },
                Err(e) => {
                    return Err(e).context("Communication Error");
                }
            }
        }
    }

    Ok(())
}
