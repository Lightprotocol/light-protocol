#!/usr/bin/env bash

set -euo pipefail
set -x  # Enable debug mode

# Constants
MAX_REQUESTS_PER_SECOND=50
FETCH_INTERVAL=$((1000000 / MAX_REQUESTS_PER_SECOND)) # in microseconds
LOG_FILE="program_metrics.log"
TEMP_FILE="temp_txs.json"
UPDATE_INTERVAL=60 # seconds
BACKFILL_HOURS=24 # configurable

# Function to display usage
usage() {
    echo "Usage: $0 <PROGRAM_ID> <RPC_URL> [BACKFILL_HOURS]"
    echo "Example: $0 Lighton6oQpVkeewmo2mcPTQQp7kYHr4fWpAgJyEmDX https://devnet.helius-rpc.com?api-key=YOUR_API_KEY 48"
    exit 1
}

# Check for required commands
check_dependencies() {
    local deps=("solana" "jq" "awk" "bc")
    for dep in "${deps[@]}"; do
        if ! command -v "$dep" &> /dev/null; then
            echo "Error: $dep is not installed. Please install it and try again."
            exit 1
        fi
    done
}

# Function to get date in ISO 8601 format
get_iso_date() {
    if [[ "$OSTYPE" == "darwin"* ]]; then
        date -u -v "${1:-+0H}" +"%Y-%m-%dT%H:%M:%SZ"
    else
        date -u -d "${1:-now}" +"%Y-%m-%dT%H:%M:%SZ"
    fi
}

# Function to fetch transactions with rate limiting
fetch_transactions() {
    local program_id=$1
    local rpc_url=$2
    local before=$3
    local after=$4
    
    echo "Fetching transactions before $before..."
    
    if [ "$before" == "latest" ]; then
        before_arg=""
    else
        before_arg="--before $before"
    fi

    if ! solana transaction-history \
        --url "$rpc_url" \
        $before_arg \
        --show-transactions \
        --commitment confirmed \
        --limit 100 \
        "$program_id" > transactions.json 2>error.log; then
        echo "Error occurred while fetching transactions:"
        cat error.log
        return 1
    fi

    if [ ! -s transactions.json ]; then
        echo "No transactions found or empty response."
        return 0
    fi

    jq -c '.[]' transactions.json | while read -r tx; do
        block_time=$(echo "$tx" | jq -r '.blockTime')
        if (( block_time >= $(date -d "$after" +%s) )); then
            echo "$tx" >> "$TEMP_FILE"
            usleep "$FETCH_INTERVAL"
        else
            break
        fi
    done

    # Return the signature of the last transaction
    last_tx=$(tail -n 1 transactions.json | jq -r '.[0].signature')
    echo "$last_tx"
}

# Function to parse transactions
parse_transactions() {
    echo "Parsing transactions..."
    jq -r '[.blockTime, .err // "success"] | @tsv' "$TEMP_FILE" | sort -n > "$LOG_FILE"
}

# Function to plot metrics
plot_metrics() {
    clear
    echo "Plotting metrics..."
    awk '
    BEGIN {
        PROCINFO["sorted_in"] = "@ind_num_asc"
    }
    {
        time=$1; status=$2;
        if (status == "success") { success[time]++ }
        else { errors[time,status]++ }
    }
    END {
        for (t in success) {
            printf "\033[0;32m%s %d\033[0m ", t, success[t];
            for (e in errors) {
                split(e, a, SUBSEP);
                if (a[1] == t) {
                    color = (a[2] == "success") ? 32 : 31;
                    printf "\033[0;%dm%s:%d\033[0m ", color, a[2], errors[e];
                }
            }
            print "";
        }
    }' "$LOG_FILE" | sort -n | tail -n 20
}

# Function to calculate TPS
calculate_tps() {
    local total_tx=$(wc -l < "$LOG_FILE")
    local time_range=$((BACKFILL_HOURS * 3600))
    local tps=$(echo "scale=2; $total_tx / $time_range" | bc)
    echo "Average TPS over last $BACKFILL_HOURS hours: $tps"
}

# Main function
main() {
    local program_id=$1
    local rpc_url=$2

    check_dependencies

    # Ensure clean start
    rm -f "$TEMP_FILE" "$LOG_FILE"

    local end_time=$(get_iso_date)
    local start_time=$(get_iso_date "-${BACKFILL_HOURS}H")
    local before="latest"

    while true; do
        echo "Fetching transactions before $before"
        last_tx=$(fetch_transactions "$program_id" "$rpc_url" "$before" "$start_time")
        
        if [ -z "$last_tx" ]; then
            echo "No more transactions to fetch."
            break
        fi

        before=$last_tx

        # Check if we've reached or passed the start time
        oldest_tx_time=$(jq -r '.[0].blockTime' transactions.json)
        if (( oldest_tx_time < $(date -d "$start_time" +%s) )); then
            break
        fi
    done

    echo "Parsing transactions..."
    parse_transactions

    echo "Plotting metrics..."
    plot_metrics

    echo "Calculating TPS..."
    calculate_tps

    echo "Script completed successfully"
}

# Check arguments
if [ $# -lt 2 ] || [ $# -gt 3 ]; then
    usage
fi

PROGRAM_ID=$1
RPC_URL=$2
BACKFILL_HOURS=${3:-24}

echo "Arguments received: PROGRAM_ID=$PROGRAM_ID, RPC_URL=$RPC_URL, BACKFILL_HOURS=$BACKFILL_HOURS"

# Run main function
main "$PROGRAM_ID" "$RPC_URL"
