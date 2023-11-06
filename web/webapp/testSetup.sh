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
trap "kill ${PID_redis} 2> /dev/null" EXIT

echo "starting solana-test-validator"
solana config set --url http://localhost:8899
sleep 1
./../../cli/test_bin/run test-validator -b > .logs/validator-logs.txt &
PID_VALIDATOR="${!}"
trap "kill ${PID_redis} 2> /dev/null; kill ${PID_VALIDATOR} 2> /dev/null" EXIT

sleep 15

echo "starting relayer server"
kill $(lsof -ti :3332) > /dev/null  || true
sleep 1

# Load the environment variables from the relayer's .env file
source ./../../relayer/.env.example

node ./../../relayer/lib/index.js > .logs/relayer-logs.txt &
PID_RELAYER="${!}"
trap "kill ${PID_redis} 2> /dev/null; kill ${PID_VALIDATOR} 2> /dev/null; kill ${PID_RELAYER} 2> /dev/null" EXIT
sleep 15
echo "running"

# Start your web application on port 3000
echo "starting web application"
pnpm serve > .logs/webapp-logs.txt &
PID_WEBAPP="${!}"
trap "kill ${PID_redis} 2> /dev/null; kill ${PID_VALIDATOR} 2> /dev/null; kill ${PID_RELAYER} 2> /dev/null; kill ${PID_WEBAPP} 2> /dev/null" EXIT
sleep 10

# Run Cypress tests
echo "running Cypress tests"
pnpm run cypress:run