use handsfreectl::cli::{Cli, Commands};
use handsfreectl::daemon::{connect_to_daemon, get_socket_path, send_command};
use handsfreectl::protocol::{DaemonCommand, DaemonResponse};

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    let socket_path = match get_socket_path() {
        Ok(path) => path,
        Err(e) => {
            eprintln!("Error determining socket path: {}", e);
            std::process::exit(1);
        }
    };

    let mut stream = match connect_to_daemon(&socket_path).await {
        Ok(stream) => stream,
        Err(e) => {
            // Make error more specific
            eprintln!("Connection Error: {}", e);
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

    // Send command and handle response
    match send_command(&mut stream, &daemon_command).await {
        Ok(response) => match response {
            DaemonResponse::Ack => {
                println!("OK"); // Simple acknowledgement for successful commands
            }
            DaemonResponse::Status(status_info) => {
                // Print state directly, error on next line if present
                println!("{}", status_info.state);
                if let Some(err) = status_info.last_error {
                    println!("{}", err);
                }
            }
            DaemonResponse::Error { message } => {
                eprintln!("Error: {}", message); // Error reported by daemon
                std::process::exit(1);
            }
        },
        Err(e) => {
            eprintln!("Communication Error: {}", e); // Error in sending/receiving/parsing
            std::process::exit(1);
        }
    }
}
