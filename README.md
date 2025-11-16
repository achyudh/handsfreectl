# handsfreectl

`handsfreectl` is the command-line control interface for the [Handsfree](https://github.com/achyudh/handsfree) speech-to-text daemon (`handsfreed`). It allows you to start and stop transcription, check the daemon's status, and more.

## Overview

This tool provides a simple way to interact with the `handsfreed` daemon from the command line, making it easy to integrate with scripts, keyboard shortcuts, or other tools in a Linux desktop environment.

## Installation

`handsfreectl` requires the `handsfreed` daemon to be installed and running. Please follow the instructions [here](https://github.com/achyudh/handsfreed/blob/main/README.md#installation) to install `handsfreed` first.

There are several ways to install `handsfreectl`:

### Pre-compiled Binaries

You can download pre-compiled binaries directly from the [GitHub Releases page](https://github.com/achyudh/handsfreectl/releases). This is a good option if you don't have Cargo installed. After downloading, make sure to place the binary in a directory that is included in your system's `PATH` environment variable.

### From Crates.io

If you have the Cargo installed, you can install `handsfreectl` from Crates.io using `cargo`:

```bash
cargo install handsfreectl
```

### Build From Source

You can build `handsfreectl` from source using Cargo.

**Prerequisites:**
* **Rust:** Version 1.85.0 or newer.

**Steps:**
1.  **Clone the Repository:**
    ```bash
    git clone https://github.com/achyudh/handsfreectl.git
    cd handsfreectl
    ```
2.  **Build:**
    ```bash
    cargo build --release
    ```
3.  **Install:**
    Copy the compiled binary to a directory that is included in your system's `PATH` environment variable.

## Usage

`handsfreectl` communicates with the `handsfreed` daemon, which must be running for these commands to work.

*   **Start Transcription:**
    Tells the daemon to start listening for speech. The transcribed text can be output as simulated keyboard input or copied to the clipboard, depending on the daemon's configuration.
    ```bash
    handsfreectl start --output keyboard
    handsfreectl start --output clipboard
    ```

*   **Stop Transcription:**
    Tells the daemon to stop the current listening session.
    ```bash
    handsfreectl stop
    ```

*   **Check Status:**
    Queries the daemon's current state.
    ```bash
    handsfreectl status
    ```
    Possible outputs include `Idle`, `Listening`, `Processing`, `Error`, or `Inactive`.

*   **Shutdown Daemon:**
    Tells the `handsfreed` process to shut down cleanly.
    ```bash
    handsfreectl shutdown
    ```

## License

This project is licensed under the GNU General Public License v3.0.
