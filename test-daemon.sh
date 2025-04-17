#!/bin/bash

# Get XDG_RUNTIME_DIR or use default
RUNTIME_DIR="${XDG_RUNTIME_DIR:-/tmp}"
SOCKET_PATH="$RUNTIME_DIR/handsfree.sock"

# Cleanup function
cleanup() {
    echo -e "\nCleaning up..."
    rm -f "$SOCKET_PATH"
    exit 0
}

# Set cleanup on script exit
trap cleanup EXIT
trap cleanup SIGINT
trap cleanup SIGTERM

# Remove socket if it already exists
rm -f "$SOCKET_PATH"

echo "Starting test daemon..."
echo "Listening on socket: $SOCKET_PATH"

# Listen on the Unix domain socket
while true; do
    nc -U -l "$SOCKET_PATH" | while read line; do
        echo "Received command: $line"
        # Parse JSON with jq if available
        if command -v jq >/dev/null 2>&1; then
            echo "Parsed JSON:"
            echo "$line" | jq '.'
        fi
    done
done
