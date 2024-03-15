#!/usr/bin/env bash

PROVER_ADDRESS="localhost:3001"
DURATION="5s"
TARGETS_FILE="targets.txt"
MEAN_TIME_THRESHOLD=150  # in milliseconds

# Initial settings
rate=10  # Start with 10 requests per second
step=5   # Increment rate by 5 in each iteration

while true; do
  BASE_OUTPUT_FILE="results_${DURATION}"
  rm -f ${BASE_OUTPUT_FILE}*

  echo "POST http://$PROVER_ADDRESS/inclusion" | vegeta attack -duration=$DURATION -rate "${rate}/1s" -targets=$TARGETS_FILE | tee $BASE_OUTPUT_FILE.bin | vegeta report
  
  mean_time=$(vegeta report $BASE_OUTPUT_FILE.bin | grep 'min, mean' | awk '{print $10}' | sed 's/ms,//')

  if (( $(echo "$mean_time > $MEAN_TIME_THRESHOLD" | bc -l) )); then
    # Threshold exceeded, reduce the rate
    rate=$((rate - step)) 
    break  # Exit the loop, we found an approximation
  else
    # Increase the rate
    rate=$((rate + step)) 
  fi
done

echo "Maximum sustainable rate (approx.): $rate requests/seconds"
