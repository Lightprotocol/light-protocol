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

if [[ $# -eq 0 ]]; then
  echo "Error: Please provide at least one argument containing light-prover options."
  echo "Allowed options: inclusion, non-inclusion, combined (individually or combined)"
  exit 1
fi

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

kill_light_prover && ./light-prover start \
  $(if [ "$inclusion" = true ]; then echo '--inclusion=true'; fi) \
  $(if [ "$non_inclusion" = true ]; then echo '--non-inclusion=true'; fi) \
  $(if [ "$combined" = true ]; then echo '--combined=true'; fi) &