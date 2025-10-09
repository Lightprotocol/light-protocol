# 📦 Light Merkle Tree Prover

Light Prover is a service for processing of Merkle tree updates.
The correctness of the batched Merkle tree update is assured through the generation of a SNARK (generated
through [gnark](https://github.com/ConsenSys/gnark)).

# Acknowledgment
This project is based on the [Semaphore Merkle Tree Batcher (SMTB)](https://github.com/worldcoin/semaphore-mtb) by Worldcoin. 
We've adapted their work to suit our specific needs. 
We encourage interested users to check out the original [SMTB](https://github.com/worldcoin/semaphore-mtb) project for reference.

## Table of Contents

1. [Usage](#usage)
2. [Benchmarks](#benchmarks)
3. [Running](#running)
4. [Docker](#docker)
5. [Contributing](#contributing)

## Usage

This part explains the existing cli commands.

1. setup - builds a circuit with provided number of compressed accounts and depth, compiles it and writes it to a file.  
   Flags:  
   1. output *file path* - A path used to output a file  
   2. tree-depth *n* - Merkle tree depth  
   3. compressedAccounts *n* - Number of COMPRESSED_ACCOUNTs
2. gen-test-params - Generates test params given the number of compressedAccounts and tree depth.
   Flags:  
   1. tree-depth *n* - Depth of the mock merkle tree  
   2. compressedAccounts *n* - Number of COMPRESSED_ACCOUNTs
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
   3. compressed-accounts *n* - number of COMPRESSED_ACCOUNTs

## Running

```shell
go build .
light-prover --config path/to/config/file
```

## Performance Testing

We have included two scripts to benchmark the performance:

`./scripts/stress_load.sh`:  This script facilitates stress testing by allowing you to define the test duration and rate.
`./scripts/rate_detection.sh`: This script is designed to detect a predetermined sustainable response rate where the mean response time does not exceed the MEAN_TIME_THRESHOLD.


Response time distribution for 30 proofs/sec on Digital Ocean droplet with 16 vCPUs:
```
Bucket           #    %       Histogram
[0s,     10ms]   0    0.00%   
[10ms,   20ms]   0    0.00%   
[20ms,   30ms]   0    0.00%   
[30ms,   40ms]   0    0.00%   
[40ms,   50ms]   7    2.33%   #
[50ms,   60ms]   268  89.33%  ###################################################################
[60ms,   70ms]   21   7.00%   #####
[70ms,   80ms]   1    0.33%   
[80ms,   90ms]   2    0.67%   
[90ms,   100ms]  1    0.33%   
[100ms,  +Inf]   0    0.00%  
```


## Unit Tests
To run specific tests cd into respective folder (merkle-tree/prover) and `go test -v -run <function-name>`

1. Integration tests
   `go test`
2. Generate csv Test Data for combined, inclusion, non-inclusion
   `cd merkle-tree && go test`
3. Unit Tests
   `cd prover && go test`

## Docker

The Docker image is configured to include only the necessary proving-key files:
- Files with `mainnet*` prefix
- Files with `inclusion*` prefix
- Files with `non-inclusion_*` prefix
- Files with `combined_*` prefix

### Building the Docker Image

```shell
# Build the Docker image with the selected proving keys
# Make sure to run this command from the prover/server directory
docker build -t light-prover .
```

### Running the Docker Image

```shell
docker run -d \
  -p 3001:3001 \
  light-prover:latest
```

### Docker Compose

```yaml
light-prover:
  # Path to the prover/server directory
  build: ./prover/server
  ports:
    # Server
    - "3001:3001"
    # Metrics
    - "9998:9998"
```

```shell
# Build and run with Docker Compose
docker compose build
docker compose up -d
```

## Formal Verification

1. Install [Elan](https://github.com/leanprover/elan).
2. ```
   cd formal-verification
   lake exe cache get # optional, but speeds up dependency compilation
   lake build         # compiles and checks the theorems
   ```