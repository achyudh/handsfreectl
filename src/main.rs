use handsfreectl::cli::Cli;
use handsfreectl::daemon::{get_socket_path, connect_to_daemon, send_command};
use handsfreectl::protocol::DaemonCommand;
use handsfreectl::cli::Commands;

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
             eprintln!("{}", e);
             std::process::exit(1);
         }
    };

    let daemon_command = match &cli.command {
        Commands::Start { output } => {
            println!("Action: Start transcription");
            DaemonCommand::Start { output_mode: output.clone() }
        }
        Commands::Stop => {
            println!("Action: Stop transcription");
            DaemonCommand::Stop
        }
    };

    if let Err(e) = send_command(&mut stream, &daemon_command).await {
        eprintln!("Error sending command: {}", e);
        std::process::exit(1);
    }

    println!("Command sent successfully to daemon.");
}
