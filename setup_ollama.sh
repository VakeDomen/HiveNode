#!/bin/bash

# Constants
HOST="0.0.0.0"
BASE_PORT=11434
OLLAMA_BINARY=""
LOG_DIR="ollama-server-logs"
SLEEP_INTERVAL=1

# Function to check and install netstat if not present
ensure_netstat() {
    if ! command -v netstat &> /dev/null; then
        echo "netstat could not be found. Attempting to install..."
        # Detect the package manager and install net-tools (which provides netstat)
        if command -v apt-get &> /dev/null; then
            sudo apt-get update && sudo apt-get install -y net-tools
        elif command -v yum &> /dev/null; then
            sudo yum install -y net-tools
        elif command -v pacman &> /dev/null; then
            sudo pacman -Sy net-tools
        else
            echo "Error: Unsupported package manager. Please install net-tools manually."
            exit 1
        fi

        if ! command -v netstat &> /dev/null; then
            echo "Error: Failed to install netstat. Please install net-tools manually."
            exit 1
        fi
    fi
}

# Function to extract the Ollama binary path using whereis
get_ollama_binary() {
    # Use whereis to find all paths related to 'ollama'
    local paths
    paths=$(whereis ollama | awk '{print $2}')

    # Iterate through the paths to find an executable file
    for path in $paths; do
        if [[ -x "$path" ]]; then
            echo "$path"
            return 0
        fi
    done

    # If no executable found, return an empty string
    echo ""
    return 1
}

# Function to find the next available port starting from BASE_PORT
find_free_port() {
    local port=$BASE_PORT
    while true; do
        if ! netstat -tuln | grep -q ":$port\b"; then
            echo "$port"
            return 0
        fi
        ((port++))
    done
}

# Function to prompt for integer input with validation
prompt_integer() {
    local prompt_msg=$1
    local input
    while true; do
        read -rp "$prompt_msg" input
        if [[ "$input" =~ ^[0-9]+$ ]] && [[ "$input" -gt 0 ]]; then
            echo "$input"
            return 0
        else
            echo "Please enter a positive integer."
        fi
    done
}

# Function to prompt for yes/no input
prompt_yes_no() {
    local prompt_msg=$1
    local input
    while true; do
        read -rp "$prompt_msg (y/n): " input
        case "$input" in
            [Yy]* ) echo "yes"; return 0;;
            [Nn]* ) echo "no"; return 0;;
            * ) echo "Please answer yes or no.";;
        esac
    done
}

# Function to prompt for GPU selection
prompt_gpu_selection() {
    local num_gpus=$1
    local selected_gpus=()

    echo "Available GPUs:"
    for ((i=0; i<num_gpus; i++)); do
        echo "  GPU $i"
    done

    echo "Enter the GPU indices you want to assign, separated by spaces (e.g., 0 2 3):"
    while true; do
        read -rp "GPU indices: " -a gpu_indices
        valid=true
        for gpu in "${gpu_indices[@]}"; do
            if ! [[ "$gpu" =~ ^[0-9]+$ ]] || (( gpu < 0 )) || (( gpu >= num_gpus )); then
                echo "Invalid GPU index: $gpu. Please enter valid indices between 0 and $((num_gpus-1))."
                valid=false
                break
            fi
        done
        if $valid; then
            selected_gpus=("${gpu_indices[@]}")
            break
        fi
    done

    echo "${selected_gpus[@]}"
}

# Ensure netstat is available
ensure_netstat

# Extract the Ollama binary path
OLLAMA_BINARY=$(get_ollama_binary)

# Check if the Ollama binary was found
if [[ -z "$OLLAMA_BINARY" ]]; then
    echo "Error: Ollama binary not found. Please ensure Ollama is installed and in your PATH."
    exit 1
fi

echo "Using Ollama binary at: $OLLAMA_BINARY"

# Prompt the user for the number of Ollama instances
NUM_INSTANCES=$(prompt_integer "Enter the number of Ollama instances you want to start: ")

# Prompt the user for the number of GPUs on the system
NUM_GPUS=$(prompt_integer "Enter the number of GPUs available on the system: ")

# GPU assignments array
declare -a GPU_ASSIGNMENTS

if [[ "$NUM_INSTANCES" -eq 1 ]]; then
    # If only one instance, ask if it should see all GPUs or specific GPU
    RESPONSE=$(prompt_yes_no "Should the single instance see all GPUs?")
    if [[ "$RESPONSE" == "yes" ]]; then
        GPU_ASSIGNMENTS=("all")
    else
        # Prompt for specific GPU
        GPU=$(prompt_gpu_selection "$NUM_GPUS")
        GPU_ASSIGNMENTS+=("$GPU")
    fi
else
    # Multiple instances: default is one instance per GPU
    # Ask if the user wants to assign GPUs manually
    RESPONSE=$(prompt_yes_no "Do you want to manually assign GPUs to each instance?")
    if [[ "$RESPONSE" == "yes" ]]; then
        for ((i=0; i<NUM_INSTANCES; i++)); do
            echo "Assigning GPU for instance $((i+1)):"
            GPU=$(prompt_gpu_selection "$NUM_GPUS")
            GPU_ASSIGNMENTS+=("$GPU")
        done
    else
        # Assign one GPU per instance by default
        for ((i=0; i<NUM_INSTANCES; i++)); do
            GPU=$((i % NUM_GPUS))
            GPU_ASSIGNMENTS+=("$GPU")
        done
    fi
fi

# Create log directory if it doesn't exist
mkdir -p "$LOG_DIR"

# Function to start an Ollama server instance
start_instance() {
    local instance_num=$1
    local port=$2
    local gpu_assignment=$3
    local log_file=$4

    # Environment variables
    export OLLAMA_LOAD_TIMEOUT="120m"
    export OLLAMA_KEEP_ALIVE="120m"
    export OLLAMA_NUM_PARALLEL="16"
    export OLLAMA_HOST="${HOST}:${port}"

    if [[ "$gpu_assignment" == "all" ]]; then
        unset CUDA_VISIBLE_DEVICES
        echo "Instance $instance_num: Using all GPUs."
    else
        export CUDA_VISIBLE_DEVICES="$gpu_assignment"
        echo "Instance $instance_num: Assigned to GPU(s) $gpu_assignment."
    fi

    # Start server with nohup and log output
    nohup "$OLLAMA_BINARY" serve > "$log_file" 2>&1 &

    if [[ $? -eq 0 ]]; then
        echo "Started server instance $instance_num on port ${port}, logging to ${log_file}"
    else
        echo "Error: Failed to start server instance $instance_num on port ${port}"
    fi
}

# Start server instances
for ((i=0; i<NUM_INSTANCES; i++)); do
    # Find a free port
    PORT=$(find_free_port)
    LOG_FILE="${LOG_DIR}/${PORT}.log"

    # Get GPU assignment
    GPU_ASSIGNMENT=${GPU_ASSIGNMENTS[i]}

    # Start the instance
    start_instance "$((i+1))" "$PORT" "$GPU_ASSIGNMENT" "$LOG_FILE"

    # Sleep interval between starting instances
    sleep "$SLEEP_INTERVAL"
done

echo "All server instances started successfully."
