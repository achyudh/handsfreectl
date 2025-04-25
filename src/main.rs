use handsfreectl::cli::{Cli, Commands};
use handsfreectl::daemon::{connect_to_daemon, get_socket_path, send_command};
use handsfreectl::protocol::{DaemonCommand, DaemonResponse};
use log::{debug, error};

#[tokio::main]
async fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("handsfreectl=warn")).init();

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
            // Make error more specific
            error!("Connection Error: {}", e);
            std::process::exit(1);
        }
    };

    // Convert CLI command to daemon command
    let daemon_command = match &cli.command {
        Commands::Start { output } => DaemonCommand::Start {
            output_mode: output.clone(),
        },
        Commands::Stop => DaemonCommand::Stop,
        Commands::Status => DaemonCommand::Status,
        Commands::Shutdown => DaemonCommand::Shutdown,
    };

    debug!("Sending command: {:?}", daemon_command);

    // Send command and handle response
    match send_command(&mut stream, &daemon_command).await {
        Ok(response) => match response {
            DaemonResponse::Ack => {
                println!("OK"); // Simple acknowledgement for successful commands
            }
            DaemonResponse::Status { status } => {
                // Print state directly, error on next line if present
                println!("{}", status.state);
                if let Some(err) = status.last_error {
                    println!("{}", err);
                }
            }
            DaemonResponse::Error { message } => {
                error!("Daemon Error: {}", message); // Error reported by daemon
                std::process::exit(1);
            }
        },
        Err(e) => {
            error!("Communication Error: {}", e); // Error in sending/receiving/parsing
            std::process::exit(1);
        }
    }
}