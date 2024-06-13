#!/usr/bin/env bash

set -x

kill_light_prover() {
  pkill -f '.*prover-.*' || true
}

build_prover() {
  cd "$root_dir/light-prover"
  go build || {
    echo "light-prover build failed. Check for errors."
    exit 1
  }
}

if [[ $# -eq 0 ]]; then
  echo "Error: Please provide at least one argument containing light-prover options."
  echo "Allowed options: inclusion, non-inclusion, combined (individually or combined)"
  exit 1
fi

root_dir=$(git rev-parse --show-toplevel 2>/dev/null) || {
  echo "Error: Not in a Git repository or 'git' command not found."
  exit 1
}

build_prover

options=("$@")
inclusion=false
non_inclusion=false
combined=false

for option in "${options[@]}"; do
  case $option in
  inclusion)
    inclusion=true
    ;;
  non-inclusion)
    non_inclusion=true
    ;;
  combined)
    combined=true
    ;;
  *)
    echo "Error: Invalid option '$option'. Allowed options: inclusion, non-inclusion, combined"
    exit 1
    ;;
  esac
done

keys_dir="$root_dir/light-prover/proving-keys/"
cmd="$root_dir/light-prover/light-prover start --keys-dir=$keys_dir"
if [ "$inclusion" = true ]; then cmd="$cmd --inclusion=true"; fi
if [ "$non_inclusion" = true ]; then cmd="$cmd --non-inclusion=true"; fi
if [ "$combined" = true ]; then cmd="$cmd --combined=true"; fi

kill_light_prover

echo "Running command: $cmd"
$cmd &
echo "Command completed with status code $?"