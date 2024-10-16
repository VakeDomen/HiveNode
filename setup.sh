#!/bin/bash

# Exit immediately if a command exits with a non-zero status
set -e

# Function to prompt user for environment variable values
prompt_env_variables() {
    echo "Please enter the following configuration values (press Enter to keep current value):"

    read -p "HIVE_CORE_URL [$HIVE_CORE_URL]: " input
    HIVE_CORE_URL=${input:-$HIVE_CORE_URL}

    read -p "HIVE_KEY [$HIVE_KEY]: " input
    HIVE_KEY=${input:-$HIVE_KEY}

    read -p "OLLAMA_URL [$OLLAMA_URL]: " input
    OLLAMA_URL=${input:-$OLLAMA_URL}

    read -p "CONCURRENT_REQUESTS [$CONCURRENT_REQUESTS]: " input
    CONCURRENT_REQUESTS=${input:-$CONCURRENT_REQUESTS}

    # Write the new values back to .env
    cat > .env <<EOF
HIVE_CORE_URL=$HIVE_CORE_URL
HIVE_KEY=$HIVE_KEY
OLLAMA_URL=$OLLAMA_URL
CONCURRENT_REQUESTS=$CONCURRENT_REQUESTS
EOF

    echo ".env file has been updated."
}

# Check if .env exists; if not, copy .env.sample to .env and prompt for modification
if [ ! -f .env ]; then
    cp .env.sample .env
    echo ".env file not found. Created from .env.sample."

    # Load variables from .env
    source .env

    # Prompt user to modify values
    prompt_env_variables
fi

# Check if ollama is installed; if not, install it
if ! command -v ollama >/dev/null 2>&1; then
    echo "ollama is not installed. Installing..."
    curl -fsSL https://ollama.com/install.sh | sudo sh
else
    echo "ollama is already installed."
fi

# Check if Rust is installed; if not, install it with default options
if ! command -v rustc >/dev/null 2>&1; then
    echo "Rust is not installed. Installing Rust..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    # Source the cargo environment to update PATH
    source $HOME/.cargo/env
else
    echo "Rust is already installed."
fi

# Run the Rust application in release mode
echo "Running the Rust application..."
cargo run --release
