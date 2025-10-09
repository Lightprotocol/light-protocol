#!/usr/bin/env bash

# Builds the verifiable solana-verify crate
solana-verify build --library-name account_compression  &&\
solana-verify build --library-name light_compressed_token &&\
solana-verify build --library-name light_system_program &&\
solana-verify build --library-name light_registry