# onchain-experiments

## program



- start a local validator

```solana-test-validator --reset```


- build & deploy program inside /program

```cd program && sh deploy_program.sh```

```cd program_prep_inputs && sh deploy_program.sh```



## CLI
Does not work with current merkletree

- set up the .env file with a private key (64-byte, i.e. PRIVATE_KEY="1,1,1,1,....")
-airdrop yourself tokens
```solana airdrop 100 <publicKey>```

- inside /webassembly build wasm binary and bindings for main.js to consume

```cd Client-Js/webassembly && sh compile_wasm.sh```
```cd Client-Js```
```npm install```

- execute CLI, commands:

run this once:
```npm run-script run init_merkle_tree ```

```npm run-script run deposit SOL 1```

```npm run-script run withdraw <note> <address_to>```

## Setup

The current tree height is set to 11. For this tree height init data and an instruction order
to insert a leaf into the tree are hardcoded. To generate a new instruction order and init data
run:
``` cd program && cargo test merkle_tree_print_init_data_and_instruction_order && cd ..```
