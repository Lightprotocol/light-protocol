# Swap 

This is a TypeScript implementation of the private swap. This example is built on the Solana blockchain and is bootstrapped using the [Light CLI](https://www.npmjs.com/package/@lightprotocol/cli) (which also leverages the Anchor framework).  

It uses [Light Protocol v3](https://github.com/Lightprotocol/light-protocol) for private state and state transitions. This allows the swaps to be executed fully on-chain.

## Prerequisites

Before running the code, ensure that you have the following installed on your machine:
– node.js, yarn
- circom
- rust
- cargo-expand (```cargo install cargo-expand```)
- solana-cli >= 1.16.4

## Setup

1. Install the required dependencies using npm or yarn:
```bash
yarn install
```

2. Build circuits:
```
yarn build
```

3. Execute the test suite using the following command:
```bash
yarn test
```

## Contributing

Contributions are welcome. Please open an issue or submit a pull request on GitHub.


## Tutorial

In this tutorial you will build a private swap which can be used to negotiate and settle an over the counter (OTC) transaction.

We will implement a private solana program (PSP).
The PSP repository 

### Primer on Light Protocol:
- Shielded Balance:
    - Sield(deposit): You can deposit(shield) value to Light Protocol. You transfer to the Light liquidity pool and receive a utxo in return.
    Example: Alice shields one sol thus receives one utxo worth 1 sol in return.
    - Utxos (Unspent transaction output) are used to store state and value.
  You can imagine a utxo similar to a bank note, which is single use and can be split up.
  Example: Alice has one utxo A which holds 1 sol.
  Alice sends Bob 0.5 sol. Utxo A is Alice's transaction input utxo. Bob receives utxo B worth 0.5. Alice receives a change utxo C worth the remaining 0.5 sol. In this transaction Utxo A 
  [See wikipedia](https://en.wikipedia.org/wiki/Unspent_transaction_output) for a more detailed explanation.
- Zero-knowledge proofs are used to prove validity of a transaction.


### Application flow

Alic wants to sell Sol and knows that Bob is a potential buyer.
**Alice:**
    1. Creates offer escrow utxo
**Bob:**
    2. Fetches offer escrow utxo
    3. Creates out utxos
    4. Generates system and PSP proofs
    5. Creates solana instructions
    6. Settles trade by invoking Swap PSP in 3 transactions



### Repo Structure:

### Steps:
1. The circuit
    1.1. Instructions
    1.2. 
2. Program (Does not need to be modified)
3. Test

1. Circuit 
    Defined in /circuits/swaps/swaps.light
    1.1. Uncomment lines 36-40

    1.2. 

## Take Offer Instruction

1. Fund Seller and buyer
2. 


## Counter Offer Instruction 


## Cancel Instruction 