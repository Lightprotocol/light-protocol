#!/bin/bash

# The program to run
PROGRAM="cargo run -- nullify-addresses"

# Number of instances to spawn
NUM_INSTANCES=2

# Start time
start_time=$(date +%s.%N)

# Spawn instances and store PIDs
pids=()
for i in $(seq 1 $NUM_INSTANCES); do
    $PROGRAM &
    pids+=($!)
done

# Wait for all instances to finish
for pid in "${pids[@]}"; do
    wait $pid
done

# End time
end_time=$(date +%s.%N)

# Calculate and print execution time
execution_time=$(echo "$end_time - $start_time" | bc)
printf "Total execution time: %.3f seconds\n" "$execution_time"