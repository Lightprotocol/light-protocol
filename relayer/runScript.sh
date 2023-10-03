#!/usr/bin/env sh

set -eux

LIMIT_LEDGER_SIZE=500000000

NOOP_PROGRAM_ID="noopb9bkMVfRPU8AsbpTUg8AQkHtKwMYZiFUjNRtMmV"
MERKLE_TREE_PROGRAM_ID="JA5cjkRJ1euVi9xLWsCJVzsRzEkT8vcC4rqw9sVAo5d6"
VERIFIER_PROGRAM_ZERO_ID="J1RRetZ4ujphU75LP8RadjXMf3sA12yC2R44CF7PmU7i"
VERIFIER_PROGRAM_STORAGE_ID="DJpbogMSrK94E1zvvJydtkqoE4sknuzmMRoutd6B7TKj"
VERIFIER_PROGRAM_ONE_ID="J85SuNBBsba7FQS66BiBCQjiQrQTif7v249zL2ffmRZc"
VERIFIER_PROGRAM_TWO_ID="2cxC8e8uNYLcymH6RTGuJs3N8fXGkwmMpw45pY65Ay86"
MOCK_VERIFIER_PROGRAM_ID="Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS"

solana config set --url http://localhost:8899
./../cli/test_bin/run test-validator -b

if [ ! -f "$.env" ]
then
    cp .env.example .env
fi

mkdir -p .logs

echo "starting redis server"
./../.local/bin/redis-server > .logs/redis-logs.txt &
PID_redis="${!}"
sleep 15
trap "kill ${PID_redis}" EXIT
# redis specific export
export ENVIRONMENT=LOCAL

pnpm ts-node lib/index.js