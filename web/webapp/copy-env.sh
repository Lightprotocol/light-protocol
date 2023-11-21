#!/usr/bin/env sh
set -eux

# copy .env.local.example to .env if .env doesn't already exist
if [ ! -f .env ]; then
    cp .env.local.example .env
    echo ".env file created from .env.local.example"
fi

sleep 2