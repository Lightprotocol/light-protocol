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
# redis specific export
export REDIS_ENVIRONMENT=LOCAL

echo "starting solana-test-validator"
solana config set --url http://localhost:8899
sleep 1
./../cli/test_bin/run test-validator -b  > .logs/validator-logs.txt
PID="${!}"
trap "kill ${PID}" EXIT

sleep 8

echo "starting relayer server"
source .env
kill $(lsof -ti :3332) > /dev/null  || true
sleep 1
node lib/index.js > .logs/relayer-logs.txt &
PID_RELAYER="${!}"
trap "kill ${PID_RELAYER} > /dev/null || true" EXIT
sleep 15
echo "executing cli tests"
cd ../cli
# export LIGHT_PROTOCOL_CONFIG=$PWD/config.json
# set invalid relayerRecipient
./test_bin/run config --relayerRecipient=AV3LnV78ezsEBZebNeMPtEcH1hmvSfUBC5Xbyrz66666
# sync valid relayer stats again
./test_bin/run config --syncRelayer
./test_bin/run config --secretKey=LsYPAULcTDhjnECes7qhwAdeEUVYgbpX5ri5zijUceTQXCwkxP94zKdG4pmDQmicF7Zbj1AqB44t8qfGE8RuUk8
./test_bin/run config --relayerRecipient=AV3LnV78ezsEBZebNeMPtEcH1hmvSfUBC5Xbyrzqbt44
./test_bin/run airdrop 50 ALA2cnz41Wa2v2EYUdkYHsg7VnKsbH1j7secM5aiP8k
./test_bin/run airdrop 50 --token=USDC ALA2cnz41Wa2v2EYUdkYHsg7VnKsbH1j7secM5aiP8k
./test_bin/run shield:sol 20
sleep 10
./test_bin/run unshield:sol 10 ALA2cnz41Wa2v2EYUdkYHsg7VnKsbH1j7secM5aiP8k
cd ../relayer
