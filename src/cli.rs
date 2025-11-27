use clap::{Parser, Subcommand, ValueEnum};
use serde::{Deserialize, Serialize};

#[derive(ValueEnum, Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CliOutputMode {
    Keyboard,
    Clipboard,
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug, PartialEq)]
pub enum Commands {
    /// Starts the transcription
    Start {
        #[arg(long, value_enum, default_value_t = CliOutputMode::Keyboard)]
        output: CliOutputMode,
    },
    /// Stops the transcription
    Stop,
    /// Toggles the transcription state (starts if idle, stops if running)
    Toggle {
        #[arg(long, value_enum)]
        output: Option<CliOutputMode>,
    },
    /// Gets the current status of the daemon
    Status,
    /// Watch for status changes
    Watch,
    /// Tells the daemon to shut down gracefully
    Shutdown,
}

impl Cli {
    pub fn parse() -> Self {
        <Self as Parser>::parse()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::error::ErrorKind;

    #[test]
    fn test_parse_start_default() {
        let args = Cli::parse_from(&["handsfreectl", "start"]);
        match args.command {
            Commands::Start { output } => assert_eq!(output, CliOutputMode::Keyboard),
            _ => panic!("Expected Start command"),
        }
    }

    #[test]
    fn test_parse_start_clipboard() {
        let args = Cli::parse_from(&["handsfreectl", "start", "--output", "clipboard"]);
        match args.command {
            Commands::Start { output } => assert_eq!(output, CliOutputMode::Clipboard),
            _ => panic!("Expected Start command"),
        }
    }

    #[test]
    fn test_parse_start_keyboard() {
        let args = Cli::parse_from(&["handsfreectl", "start", "--output", "keyboard"]);
        match args.command {
            Commands::Start { output } => assert_eq!(output, CliOutputMode::Keyboard),
            _ => panic!("Expected Start command"),
        }
    }

    #[test]
    fn test_parse_toggle() {
        let args = Cli::parse_from(&["handsfreectl", "toggle"]);
        match args.command {
            Commands::Toggle { output } => assert_eq!(output, None),
            _ => panic!("Expected Toggle command"),
        }
    }

    #[test]
    fn test_parse_toggle_with_output() {
        let args = Cli::parse_from(&["handsfreectl", "toggle", "--output", "clipboard"]);
        match args.command {
            Commands::Toggle { output } => assert_eq!(output, Some(CliOutputMode::Clipboard)),
            _ => panic!("Expected Toggle command"),
        }
    }

    #[test]
    fn test_parse_stop() {
        let args = Cli::parse_from(&["handsfreectl", "stop"]);
        assert_eq!(args.command, Commands::Stop);
    }

    #[test]
    fn test_parse_status() {
        let args = Cli::parse_from(&["handsfreectl", "status"]);
        assert_eq!(args.command, Commands::Status);
    }

    #[test]
    fn test_parse_watch() {
        let args = Cli::parse_from(&["handsfreectl", "watch"]);
        assert_eq!(args.command, Commands::Watch);
    }

    #[test]
    fn test_parse_shutdown() {
        let args = Cli::parse_from(&["handsfreectl", "shutdown"]);
        assert_eq!(args.command, Commands::Shutdown);
    }

    #[test]
    fn test_parse_invalid_command() {
        let result = Cli::try_parse_from(&["handsfreectl", "invalid_command"]);
        match result.unwrap_err().kind() {
            ErrorKind::InvalidSubcommand => (), // Test passes
            other => panic!("Expected InvalidSubcommand error, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_invalid_output_mode() {
        let result = Cli::try_parse_from(&["handsfreectl", "start", "--output", "invalid"]);
        match result.unwrap_err().kind() {
            ErrorKind::InvalidValue => (), // Test passes
            other => panic!("Expected InvalidValue error, got {:?}", other),
        }
    }
}
