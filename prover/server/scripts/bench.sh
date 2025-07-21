#!/usr/bin/env sh

DEPTH="26"
URL="http://localhost:3001/prove"

gnark() {
    local args=("$@")
    ./light-prover "${args[@]}"
}

generate_and_test() {
    local compressedAccounts=$1
    mkdir -p circuits
    CIRCUIT_FILE="/tmp/circuit_${DEPTH}_${compressedAccounts}.key"
    TEST_FILE="/tmp/inputs_${DEPTH}_${compressedAccounts}.json"
    if [ ! -f "${CIRCUIT_FILE}" ]; then
        echo "Prover setup..."
        gnark setup --circuit inclusion --compressedAccounts "$compressedAccounts" --tree-depth "$DEPTH" --output "${CIRCUIT_FILE}"
    fi
    if [ ! -f "${TEST_FILE}" ]; then
        echo "Generating test inputs..."
        gnark gen-test-params --compressedAccounts "$compressedAccounts" --tree-depth "$DEPTH" > "${TEST_FILE}"
    fi
}

run_benchmark() {
    local compressedAccounts=$1
    echo "Benchmarking with $compressedAccounts compressedAccounts..."
    TEST_FILE="/tmp/inputs_${DEPTH}_${compressedAccounts}.json"
    curl -s -S -X POST -d @"${TEST_FILE}" "$URL" -o /dev/null
    sleep 1
}

start_server() {
  compressedAccounts_arr=$1
  for compressedAccounts in "${compressedAccounts_arr[@]}"
  do
    keys_file+="--keys-file /tmp/circuit_${DEPTH}_${compressedAccounts}.key "
  done
  echo "Starting server with keys: $keys_file"
  gnark start \
  $keys_file \
  --json-logging \
  >> log.txt \
  &
  sleep 10
}

# Define an array containing the desired values
declare -a compressedAccounts_arr=("1" "2" "3" "4" "8")

# Kill the server
killall light-prover

# Generate keys and test inputs
for compressedAccounts in "${compressedAccounts_arr[@]}"
do
    generate_and_test $compressedAccounts
done

# Start the server
start_server "${compressedAccounts_arr[@]}"

# Run the benchmarks
for compressedAccounts in "${compressedAccounts_arr[@]}"
do
    run_benchmark $compressedAccounts
done
echo "Done. Benchmarking results are in log.txt."

# Kill the server
killall light-prover