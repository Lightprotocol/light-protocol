#!/usr/bin/env sh
set -eux

if [ ! -f .env ]; then
    cp .env.local.example .env
    sync .env
    echo ".env file created from .env.local.example"
fi
