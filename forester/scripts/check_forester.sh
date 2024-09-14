#!/bin/bash

FORESTER_PATH="../target/release/forester"

if [ -f "$FORESTER_PATH" ]; then
    echo "Local Forester build found."
    $FORESTER_PATH --version
else
    echo "Local Forester build not found. Building..."
    cargo build --release
    if [ $? -eq 0 ]; then
        echo "Forester built successfully."
        $FORESTER_PATH --version
    else
        echo "Failed to build Forester. Please build it manually."
        exit 1
    fi
fi