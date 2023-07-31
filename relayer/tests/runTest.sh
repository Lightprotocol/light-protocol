#!/usr/bin/env sh

set -eux

if [ ! -f "$.env" ]
then
    cp .env.example .env
fi

mkdir -p .logs

echo "starting redis server"
redis-server > .logs/redis-logs.txt &
PID_redis="${!}"
sleep 15
trap "kill ${PID_redis}" EXIT
# redis specific export
export ENVIRONMENT=LOCAL

echo "starting solana-test-validator"
solana config set --url http://localhost:8899
sleep 1
./../cli/test_bin/run test-validator -b
PID="${!}"
trap "kill ${PID}" EXIT

sleep 8

echo "starting relayer server"
kill $(lsof -ti :3331) > /dev/null  || true
sleep 1
node lib/index.js > .logs/relayer-logs.txt &

sleep 15

echo "executing functional tests"

npx ts-mocha -p ./tsconfig.json -t 1000000 tests/functional_test.ts --exit;

kill $(lsof -ti :3331) > /dev/null  || true