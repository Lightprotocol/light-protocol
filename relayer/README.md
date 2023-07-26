# Relayer

This relayer implementation is an express server with a redis data base and bull mq queue.
The server combines relaying transactions and indexing of past transactions.

## API Endpoints

- **GET** `/getBuiltMerkletree`: This endpoint is used to retrieve a built Merkle Tree. (currently not used)
- **GET** `/lookuptable`: This endpoint is used to retrieve a Lookup Table from the relayer. (currently not used)
- **GET** `/indexedTransactions`: This endpoint is used to retrieve indexed transactions.
- **POST** `/relayTransaction`: This endpoint is used to handle relay shielded transfer and unshield requests.
- **POST** `/updatemerkletree`: This endpoint is used to update the Merkle Tree.

## Prerequisites

- redis (you can install redis for testing with ./scripts/install.sh true)

## Run

### local

Starts a local test validator, redis data base and relayer.

```
yarn build
yarn start-local
```

### testnet

Adjust .env file with correct rpc url (not tested yet)

```
yarn build
yarn start
```

## Test

- make sure you installed the dev environment with the true flag to install redis

```
yarn build
yarn test
```
