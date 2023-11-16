#!/usr/bin/env sh
set -eux
if [ ! -f ".env" ]
then
    cp .env.local.example .env
fi

mkdir -p .logs



echo "starting redis server"
redis-server > .logs/redis-logs.txt &
PID_redis="${!}"
sleep 15
trap 'if ps -p ${PID_redis} > /dev/null; then kill ${PID_redis}; fi' EXIT



echo "starting solana-test-validator"
solana config set --url http://localhost:8899
sleep 1
./../../cli/test_bin/run test-validator -b > .logs/validator-logs.txt &
PID_VALIDATOR="${!}"
sleep 15
trap 'if ps -p ${PID_redis} > /dev/null; then kill ${PID_redis}; fi; if ps -p ${PID_VALIDATOR} > /dev/null; then kill ${PID_VALIDATOR}; fi' EXIT




# Load the environment variables from the relayer's .env file
echo "building and starting relayer server"
kill $(lsof -ti :3332) > /dev/null  || true
sleep 1
echo "Current directory: $(pwd)"
ls -la
echo "perms:"
ls -l ./../../relayer/.env.example
chmod +r ./../../relayer/.env.example
. ./../../relayer/.env.example

# build the relayer
cd ./../../relayer
pnpm install
pnpm build

node ./lib/index.js > ../web/webapp/.logs/relayer-logs.txt &
cd ../web/webapp
PID_RELAYER="${!}"
sleep 15
echo "running"
trap 'if ps -p ${PID_redis} > /dev/null; then kill ${PID_redis}; fi; if ps -p ${PID_VALIDATOR} > /dev/null; then kill ${PID_VALIDATOR}; fi; if ps -p ${PID_RELAYER} > /dev/null; then kill ${PID_RELAYER}; fi' EXIT




# Start your web application on port 3000
echo "starting web application"
pnpm serve > .logs/webapp-logs.txt &
PID_WEBAPP="${!}"

# Wait for server to start
echo "waiting 90s for server to start"
sleep 90

trap 'if ps -p ${PID_redis} > /dev/null; then kill ${PID_redis}; fi; if ps -p ${PID_VALIDATOR} > /dev/null; then kill ${PID_VALIDATOR}; fi; if ps -p ${PID_RELAYER} > /dev/null; then kill ${PID_RELAYER}; fi; if ps -p ${PID_WEBAPP} > /dev/null; then kill ${PID_WEBAPP}; fi' EXIT

# Check server logs
echo ">>>>>>> server logs:"
cat .logs/webapp-logs.txt
echo "<<<<<<< server logs end"



# Check server response
echo "server response:"
curl http://localhost:3000



echo "Listing all folders/files in ./public:"
ls -la ./public
echo "Listing all folders/files in root:"
ls -la /

export TERM=xterm

echo "running Cypress tests"
pnpm run cypress:run