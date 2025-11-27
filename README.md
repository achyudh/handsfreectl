# handsfreectl

[![CI](https://github.com/achyudh/handsfreectl/actions/workflows/ci.yml/badge.svg)](https://github.com/achyudh/handsfreectl/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/handsfreectl.svg)](https://crates.io/crates/handsfreectl)
[![License](https://img.shields.io/badge/License-GPLv3-blue.svg)](https://www.gnu.org/licenses/gpl-3.0)

`handsfreectl` is a CLI for the [Handsfree](https://github.com/achyudh/handsfree) speech-to-text daemon (`handsfreed`). It allows you to start and stop transcription, check the daemon's status, and more. The goal of `handsfreectl` is to provide a simple way to interact with the `handsfreed` daemon from the command line, making it easy to integrate with scripts, keyboard shortcuts, or other tools in a Linux desktop environment.

## Installation

`handsfreectl` requires the `handsfreed` daemon to be installed and running. Please follow the [instructions](https://github.com/achyudh/handsfreed/blob/main/README.md#installation) to install `handsfreed` first.

There are several ways to install `handsfreectl`:

### Pre-compiled Binaries

You can download pre-compiled binaries directly from the [GitHub Releases page](https://github.com/achyudh/handsfreectl/releases). This is a good option if you don't have Cargo installed. After downloading, make sure to place the binary in a directory that is included in your system's `PATH` environment variable.

### From Crates.io

If you have the Cargo installed, you can install `handsfreectl` from Crates.io using `cargo`:

```bash
cargo install handsfreectl
```

### Nix Flake

If you use the [Nix package manager](https://nixos.org/) with flakes enabled, there is a [Handsfree flake](https://github.com/achyudh/handsfree) that provides the `handsfreectl` and `handsfreed` packages along with a Home Manager module to configure and manage the `handsfreed` daemon as a systemd service.

For detailed instructions on how to add the flake to your system and configure the service, please refer to the **[Handsfree flake readme](https://github.com/achyudh/handsfreed/blob/main/README.md)**.

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

*   **Toggle Transcription:**
    Toggles the transcription state. If `Idle`, it starts listening. If `Listening`, it stops. This is ideal for binding to a single hotkey.
    ```bash
    handsfreectl toggle
    handsfreectl toggle --output clipboard
    ```

*   **Check Status:**
    Queries the daemon's current state once.
    ```bash
    handsfreectl status
    ```

*   **Watch Status:**
    Streams status updates in real-time. This is efficient for status bars (like Waybar or Polybar) as it avoids polling.
    ```bash
    handsfreectl watch
    ```
    Possible outputs include `Idle`, `Listening`, `Processing`, `Error`, or `Inactive`.

*   **Shutdown Daemon:**
    Tells the `handsfreed` process to shut down cleanly.
    ```bash
    handsfreectl shutdown
    ```

## License

This project is licensed under the GNU General Public License v3.0.
