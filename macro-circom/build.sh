#!/bin/bash

# Build your project
cargo build --release

# Get the package name
pkg_name=$(grep 'name =' Cargo.toml | sed 's/name = "\(.*\)"/\1/' | xargs)

# The path where you want to save the binary
destination_dir="../cli/bin"

# Create the directory if it doesn't exist
mkdir -p $destination_dir

# Get the path to the binary
binary_path=../target/release/$pkg_name

# Copy the binary
cp $binary_path $destination_dir
