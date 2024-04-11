#!/usr/bin/env bash

kill_light_prover() {
  pkill -f 'light-prover' || true
}

# Get the root directory of the Git repository (robust error handling)
root_dir=$(git rev-parse --show-toplevel 2>/dev/null) || {
  echo "Error: Not in a Git repository or 'git' command not found."
  exit 1
}

cd "$root_dir/gnark-prover"

go build || {
  echo "Build failed. Check for errors."
  exit 1
}

if [[ $# -ne 1 ]]; then
  echo "Error: Please provide a single argument containing light-prover options."
  echo "Allowed options: inclusion, non-inclusion, combined (individually or combined)"
    exit 1
fi

options=($1)
inclusion=""
non_inclusion=""
combined=""

for option in "${options[@]}"; do
  case $option in
    inclusion)
      inclusion="--inclusion=true"
      ;;
    non-inclusion)
      non_inclusion="--non-inclusion=true"
      ;;
    combined)
      combined="--combined=true"
      ;;
    *)
      echo "Error: Invalid option '$option'. Allowed options: inclusion, non-inclusion, combined"
      exit 1
      ;;
  esac
done

kill_light_prover && ./light-prover start $inclusion $non_inclusion $combined &
light_prover_pid=$!

health_check_url="http://localhost:3001/health"
timeout=120
interval=2

start_time=$(date +%s)

while true; do
  status_code=$(curl -s -o /dev/null -w "%{http_code}" "$health_check_url")

  if [[ "$status_code" -eq 200 ]]; then
    echo "light-prover health check successful!"
    break
  fi

  current_time=$(date +%s)
  if (( current_time - start_time >= timeout )); then
    echo "light-prover failed to start within $timeout seconds."
    kill_light_prover
    exit 1
  fi

  sleep "$interval"
done