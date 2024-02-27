# 📦 Light Merkle Tree Prover

Light Prover is a service for processing of Merkle tree updates.
The correctness of the batched Merkle tree update is assured through the generation of a SNARK (generated
through [gnark](https://github.com/ConsenSys/gnark)).

## Table of Contents

1. [Usage](#usage)
2. [Benchmarks](#benchmarks)
3. [Running](#running)
4. [Docker](#docker)
5. [Contributing](#contributing)

## Usage

This part explains the existing cli commands.

1. setup - builds a circuit with provided number of utxos and depth, compiles it and writes it to a file.  
   Flags:  
   1. output *file path* - A path used to output a file  
   2. tree-depth *n* - Merkle tree depth  
   3. utxos *n* - Number of UTXOs
2. gen-test-params - Generates test params given the number of utxos and tree depth.
   Flags:  
   1. tree-depth *n* - Depth of the mock merkle tree  
   2. utxos *n* - Number of UTXOs
3. start - starts a api server with /prove and /metrics endpoints  
   Flags:
   1. config: Config file, which may contain the following fields:
   1. keys *[string]* - String array of keys file paths  
   2. keys-file *file path* - Proving system file, can be used instead of config       
   3. Optional: json-logging *0/1* - Enables json logging  
   4. Optional: prover-address *address* - Address for the prover server, defaults to localhost:3000
   5. Optional: metrics-address *address* - Address for the metrics server, defaults to localhost:9998
4. prove - Reads a prover system file, generates and returns proof based on prover parameters  
   Flags:  
   1. config: Config file, which may contain the following fields:
   1. keys *[string]* - String array of keys file paths  
   2. keys-file *file path* - Proving system file, can be used instead of config
5. verify - Takes a hash of all public inputs and verifies it with a prover system  
   Flags:  
   1. keys-file *file path* - Proving system file  
   2. input-hash *hash* - Hash of all public inputs
6. r1cs - Builds an r1cs and writes it to a file  
   Flags:  
   1. output *file path* - File to be written to  
   2. tree-depth *n* - Depth of a tree  
   3. batch-size *n* - Batch size for Merkle tree updates
7. extract-circuit - Transpiles the circuit from gnark to Lean
   Flags:  
   1. output *file path* - File to be writen to
   2. tree-depth *n* - Merkle tree depth  
   3. batch-size *n* - Batch size for Merkle tree updates

## Running

```shell
go build .
light-prover --config path/to/config/file
```

## Docker

```shell
docker build -t light-prover .

# /host/path/to/keys should contain the config file
docker run -it \
    --mount type=bind,source=host/path/to/config,target=/config \
    -p 3001:3001 \
    light-prover
```

Or in docker compose

```yaml
light-prover:
  # Path to the repo root directory
  build: ./light-prover
  volumes:
    - /host/path/to/config:/config
  ports:
    # Server
    - "3001:3001"
    # Metrics
    - "9998:9998"

  docker compose build
  docker compose up -d
```