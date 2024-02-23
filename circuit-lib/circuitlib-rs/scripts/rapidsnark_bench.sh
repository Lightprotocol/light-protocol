#!/bin/bash

LOG_FILE="execution_times.log"
echo "Benchmarking started..." >> $LOG_FILE

for ((i=1; i<10; i++))
do
    echo "Running iteration $i"
    work_dir="test-data/merkle22_$i"
    start_time=$(date +%s%6N)
    prover "$work_dir/circuit.zkey" "$work_dir/22_$i.wtns" "$work_dir/proof_merkle22_$i.json" "$work_dir/public_inputs_merkle22_$i.json"
    sleep 1
    end_time=$(date +%s%6N)
    execution_time=$(echo "scale=3; ($end_time - $start_time) / 1000 - 1000" | bc)
    echo "Iteration $i took $execution_time milliseconds" >> $LOG_FILE
done
