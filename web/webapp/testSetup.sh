#!/usr/bin/env sh
set -eux
if [ ! -f "$.env" ]
then
    cp .env.local.example .env
fi


mkdir -p .logs

echo "starting redis server"
redis-server > .logs/redis-logs.txt &
PID_redis="${!}"
sleep 15
trap "kill ${PID_redis}" EXIT

echo "starting solana-test-validator"
solana config set --url http://localhost:8899
sleep 1
./../../cli/test_bin/run test-validator -b > .logs/validator-logs.txt 
PID_VALIDATOR="${!}"
trap "kill ${PID_VALIDATOR}" EXIT

sleep 15

echo "starting relayer server"
kill $(lsof -ti :3332) > /dev/null  || true
sleep 1

# Load the environment variables from the relayer's .env file
source ./../../relayer/.env.example

node ./../../relayer/lib/index.js > .logs/relayer-logs.txt &
PID_RELAYER="${!}"
trap "kill ${PID_RELAYER} > /dev/null || true" EXIT
sleep 15
echo "running"
# pnpm cypress:open
# pnpm cypress run --spec cypress/e2e/actions/actions.cy.js
pnpm run cypress:run