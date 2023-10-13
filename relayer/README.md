# Relayer

This is a basic Relayer reference implementation. It's a simple express server with a redis database and bullmq queue to handle relay jobs.

Currently handles both relaying Light Protocol transactions and indexing of transaction events. The code is not optimized.

Relayer nodes are trustless RPCs in the Light Protocol network that receive zero knowledge proofs (bytes) from clients, pack them into Solana transactions, and forward them to the Solana validator network in exchange for a SOL reward (relayerFee). An integrity hash in the ZKP ensures trustlessness.

## API Endpoints

- **POST** `/relayTransaction`: Handles relay requests for shielded txs and unshields.
- **POST** `/updatemerkletree`: Updates the Merkle Tree.
- **GET** `/indexedTransactions`: Returns indexed txs. No pagination currently.
- **GET** `/getRelayerInfo`: Returns config of self.
- **GET** `/lookuptable`: Returns lookuptable of self. (currently not used)
- **GET** `/getBuiltMerkletree`: Returns built merkletree. (currently not used)

## Prerequisites

- redis (Redis is required. You can install Redis for testing with `./scripts/install.sh true`)

## Run

### local

Starts a local test validator, redis data base and relayer.

```
pnpm build
pnpm start-local
```

### devnet

Adjust the .env file with the following parameters:

- `RPC_URL` (of your solana rpc provider)
- `NETWORK` ('testnet' | 'devnet') Note: Light v3 is not deployed on Mainnet yet.
- `KEY_PAIR` (relayer signer secretkey)
- `RELAYER_RECIPIENT` (sol rewards collector)
- `LOOK_UP_TABLE` (pubkey as base58 or byte array)
- `LOCAL_TEST_ENVIRONMENT=false`

and if you're using a hosted DB:

- the necessary credentials (e.g. `PASSWORD`, `HOSTNAME`, `USERNAME`, `DB_PORT`)
- `REDIS_ENVIRONMENT=PROD`

> **Note:** It's important that you ensure the following:

- Ensure that your provided `LOOK_UP_TABLE` pubkey is initialized. `pnpm ts-node ./scripts/createNewLookUpTable.ts`
- Your relayer keys (`KEY_PAIR` and `RELAYER_RECIPIENT`) are both funded. `KEY_PAIR` is used as the SOL feepayer for all relayed transactions and merkle tree updates. `RELAYER_RECIPIENT` must have enough SOL to be deemed rent-exempt. `pnpm ts-node ./scripts/fundRelayer.ts`

```
pnpm build
pnpm start
```

## Test

Ensure that you have installed the dev environment with the 'true' flag to install Redis.

```
pnpm build
pnpm test
```

> **Note:** Running local tests will overwrite your .env variable for `LOOK_UP_TABLE` with a default value that is pre-initialized with the local light test-validator.
