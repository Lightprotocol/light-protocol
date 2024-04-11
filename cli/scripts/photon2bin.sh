#!/usr/bin/env sh

set -eux

root_dir="$(git rev-parse --show-toplevel)";
cli_dir="${root_dir}/cli"

cargo install --force --root "$cli_dir" photon-indexer
