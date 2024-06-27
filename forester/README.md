# Light Forester

## Description

Forester is a service for nullifying the state and address merkle trees.
It subscribes to the nullifier queue and nullifies merkle tree leaves.

## Configuration

Forester requires a configuration file, `forester.toml`, specifying necessary keys:
- `STATE_MERKLE_TREE_PUBKEY`: Address of the State Merkle tree.
- `NULLIFIER_QUEUE_PUBKEY`: Address of the State Nullifier queue.
- `ADDRESS_MERKLE_TREE_PUBKEY`: Address of the Address Merkle tree.
- `ADDRESS_MERKLE_TREE_QUEUE_PUBKEY`: Address of the Address queue.
- `REGISTRY_PUBKEY`: Address of the Registry program.


To setup your environment properly, copy `.env.example` to `.env` 
and update the `FORESTER_PAYER` field with your appropriate key. 

Alternatively, if you prefer to use a terminal profile file, 
add the key to your `~/.zshrc` (zsh) or `~/.bashrc` (bash) 
by including this line: `export FORESTER_PAYER=your_value_here`.
Substitute `your_value_here` with your actual key. 

Remember to restart your terminal or source your terminal profile for the changes to take effect.

## Usage

1. Run the service:
To subscribe to nullify the state merkle tree, use the following command:
`cargo run -- subscribe`
2. To manually nullify state merkle tree leaves, use the following command:
`cargo run -- nullify-state`
3. To manually nullify address merkle tree leaves, use the following command:
`cargo run -- nullify-addresses`
4. To manually nullify state *and* address merkle tree leaves, use the following command:
   `cargo run -- nullify`


## TODO

1. Add indexer URL to the configuration file.
2. Add address merkle tree support.
3. Add multiple merkle trees support.
