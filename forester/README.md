# Light Forester

## Description

Forester is a service for nullifying the state merkle trees.
It subscribes to the nullifier queue and nullifies the state merkle tree leaves.

## Configuration

Forester requires a configuration file, `forester.toml`, specifying necessary keys:
- `merkle_tree`: Address of the State Merkle tree.
- `nullifier_queue`: Address of the Nullifier queue.
- `payer`: The key pair for the payer.

## Usage

1. Run the service:
To subscribe to nullify the state merkle tree, use the following command:
`cargo run -- subscribe`

2. To manually nullify state merkle tree leaves, use the following command:
`cargo run -- nullify`


## TODO

1. Add indexer URL to the configuration file.
2. Add address merkle tree support.
3. Add multiple merkle trees support.
