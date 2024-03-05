#!/usr/bin/env sh

DEPTH="26"
URL="http://localhost:3001/prove"

gnark() {
    local args=("$@")
    ./light-prover "${args[@]}"
}

generate_and_test() {
    local utxos=$1
    mkdir -p circuits
    CIRCUIT_FILE="/tmp/circuit_${DEPTH}_${utxos}.key"
    TEST_FILE="/tmp/inputs_${DEPTH}_${utxos}.json"
    if [ ! -f "${CIRCUIT_FILE}" ]; then
        echo "Prover setup..."
        gnark setup --utxos "$utxos" --tree-depth "$DEPTH" --output "${CIRCUIT_FILE}"
    fi
    if [ ! -f "${TEST_FILE}" ]; then
        echo "Generating test inputs..."
        gnark gen-test-params --utxos "$utxos" --tree-depth "$DEPTH" > "${TEST_FILE}"
    fi
}

run_benchmark() {
    local utxos=$1
    echo "Benchmarking with $utxos utxos..."
    TEST_FILE="/tmp/inputs_${DEPTH}_${utxos}.json"
    curl -s -S -X POST -d @"${TEST_FILE}" "$URL" -o /dev/null
    sleep 1
}

start_server() {
#  local -n arr=$1
#  for utxos in "${arr[@]}"
  utxos_arr=$1
  for utxos in "${utxos_arr[@]}"
  do
    keys_file+="--keys-file /tmp/circuit_${DEPTH}_${utxos}.key "
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
declare -a utxos_arr=("1" "2" "3" "4" "8")

# Kill the server
killall light-prover

# Generate keys and test inputs
for utxos in "${utxos_arr[@]}"
do
    generate_and_test $utxos
done

# Start the server
start_server "${utxos_arr[@]}"

# Run the benchmarks
for utxos in "${utxos_arr[@]}"
do
    run_benchmark $utxos
done
echo "Done. Benchmarking results are in log.txt."

# Kill the server
killall light-prover