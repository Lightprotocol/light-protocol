#!/bin/bash

ROOT_DIR=$(git rev-parse --show-toplevel)
ACCOUNTS_DIR=$ROOT_DIR/cli/accounts

NULLIFIER_QUEUE_JSON=nullifier_queue_44J4oDXpjPAbzHCSc24q7NEiPekss4sAbLd8ka4gd9CZ.json
MERKLE_TREE_JSON=merkle_tree_5bdFnXU47QjzGpzHfXnxcEi5WXyxzEAZzd1vrE39bf1W.json

rm $ACCOUNTS_DIR/$NULLIFIER_QUEUE_JSON
rm $ACCOUNTS_DIR/$MERKLE_TREE_JSON

solana account 44J4oDXpjPAbzHCSc24q7NEiPekss4sAbLd8ka4gd9CZ --url http://localhost:8899 --output json > $ACCOUNTS_DIR/$NULLIFIER_QUEUE_JSON
solana account 5bdFnXU47QjzGpzHfXnxcEi5WXyxzEAZzd1vrE39bf1W --url http://localhost:8899 --output json > $ACCOUNTS_DIR/$MERKLE_TREE_JSON