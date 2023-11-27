#!/usr/bin/env sh
set -eux
if [ ! -f ".env" ]
then
    cp .env.example .env
fi
mkdir -p .logs

echo "starting redis server"
redis-server > .logs/redis-logs.txt &
PID_redis="${!}"
sleep 15
trap "kill ${PID_redis}" EXIT


export REDIS_ENVIRONMENT=LOCAL

echo "starting solana-test-validator"
solana config set --url http://localhost:8899
sleep 1
./../cli/test_bin/run test-validator -b > .logs/validator-logs.txt 
PID_VALIDATOR="${!}"
trap "kill ${PID_VALIDATOR}" EXIT

sleep 15
echo "Current directory: $(pwd)"
ls -la
echo "perms:"
ls -l .env.example
chmod +r .env.example
echo "Current directory: $(pwd)"

echo "building relayer"
pnpm install
pnpm build

echo "starting relayer server"
kill $(lsof -ti :3332) > /dev/null  || true
sleep 1
node lib/index.js > .logs/relayer-logs.txt &
PID_RELAYER="${!}"
trap "kill ${PID_RELAYER} > /dev/null || true" EXIT
sleep 15
echo "executing functional tests"

##
npx ts-mocha -p ./tsconfig.test.json -t 1000000 tests/functional_test.ts --exit;

echo "executing browser env tests"
sleep 3

npx mocha --package ./package.json -r ts-node/register -r jsdom-global/register -r ./setup.jsdom.ts tests/browser_test.ts --timeout 1000000 --exit;
