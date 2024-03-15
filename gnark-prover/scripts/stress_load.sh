#!/usr/bin/env bash

PROVER_ADDRESS="localhost:3001"
DURATION="60s"
RATE="30/1s"
TARGETS_FILE="targets.txt"
BASE_OUTPUT_FILE="results_${DURATION}"

if [ -e "${BASE_OUTPUT_FILE}*" ]; then
  rm ${BASE_OUTPUT_FILE}*
fi

echo "POST http://$PROVER_ADDRESS/inclusion" | vegeta attack -duration=$DURATION -rate $RATE -targets=$TARGETS_FILE | tee $BASE_OUTPUT_FILE.bin | vegeta report 

vegeta report $BASE_OUTPUT_FILE.bin >> $BASE_OUTPUT_FILE.txt
vegeta report -type="hist[0,10ms,20ms,30ms,40ms,50ms,60ms,70ms,80ms,90ms,100ms,110ms,120ms,130ms,140ms,150ms,160ms,170ms,180ms,190ms,200ms]" $BASE_OUTPUT_FILE.bin >> $BASE_OUTPUT_FILE.txt
vegeta plot -title=Results $BASE_OUTPUT_FILE.bin > $BASE_OUTPUT_FILE.html
