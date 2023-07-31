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

echo "executing cli tests"
cd ../cli
./test_bin/run airdrop 50 ALA2cnz41Wa2v2EYUdkYHsg7VnKsbH1j7secM5aiP8k

./test_bin/run airdrop 50 --token=USDC ALA2cnz41Wa2v2EYUdkYHsg7VnKsbH1j7secM5aiP8k

./test_bin/run shield:sol 20

./test_bin/run unshield:sol 10 ALA2cnz41Wa2v2EYUdkYHsg7VnKsbH1j7secM5aiP8k
cd ../relayer
kill $(lsof -ti :3331) > /dev/null  || true