#!/bin/bash

# 1. builds zk.js with local circuit-lib and prover.js instead of workspace dependencies
# 2. builds rpc with local zk.js instead of workspace dependency
# 3. builds docker image (consumed in deployRpc.sh)

set -eux

generate_temp_package_json() {
    dir=$1
    shift
    json=$(cat $dir/package.json)
    while [ $# -gt 0 ]; do
        dep=$1
        path=$2
        json=$(echo "$json" | jq --arg dep "$dep" --arg path "$path" 'if .dependencies[$dep] then .dependencies[$dep] = $path else . end | if .devDependencies[$dep] then .devDependencies[$dep] = $path else . end | del(.scripts.preinstall)')
        shift 2
    done
    echo "$json" > $dir/temp.package.json
}

top_dir=`git rev-parse --show-toplevel`

(cd $top_dir/account.rs && pnpm pack)
account_rs_tgz=$(ls $top_dir/account.rs/*.tgz)

(cd $top_dir/prover.js && pnpm pack)
prover_tgz=$(ls $top_dir/prover.js/*.tgz)

cleanup() {
    echo "Deleting .tgz files..."
    rm -f $top_dir/zk.js/*.tgz
    rm -f $top_dir/circuit-lib/circuit-lib.js/*.tgz
    rm -f $top_dir/prover.js/*.tgz
    rm -f $top_dir/account.rs/*.tgz

    echo "Restoring original package.json files..."

    if [ -f $top_dir/zk.js/package.json.bak ]; then
      mv -f $top_dir/zk.js/package.json.bak $top_dir/zk.js/package.json
    fi

    if [ -f $top_dir/rpc/package.json.bak ]; then
      mv -f $top_dir/rpc/package.json.bak $top_dir/rpc/package.json
    fi

    if [ -f $top_dir/account.rs/package.json.bak ]; then
      mv -f $top_dir/account.rs/package.json.bak $top_dir/account.rs/package.json
    fi

    if [ -f $top_dir/circuit-lib/circuit-lib.js/package.json.bak ]; then
      mv -f $top_dir/circuit-lib/circuit-lib.js/package.json.bak $top_dir/circuit-lib/circuit-lib.js/package.json
    fi

    echo "Deleting node_modules, cached files, and lock files..."

    rm -rf $top_dir/zk.js/node_modules
    rm -rf $top_dir/rpc/node_modules
    rm -rf $top_dir/account.rs/node_modules
    rm -rf $top_dir/circuit-lib/circuit-lib.js/node_modules
    rm -f $top_dir/pnpm-lock.yaml

    echo "Deleting NPM artifacts..."
    rm -rf $top_dir/zk.js/package-lock.json
    rm -rf $top_dir/rpc/package-lock.json
    rm -rf $top_dir/account.rs/package-lock.json
    rm -rf $top_dir/circuit-lib/circuit-lib.js/package-lock.json
    rm -rf $top_dir/pnpm-lock.yaml
}

trap cleanup EXIT

generate_temp_package_json $top_dir/circuit-lib/circuit-lib.js "@lightprotocol/account.rs" "file:$account_rs_tgz"
mv $top_dir/circuit-lib/circuit-lib.js/package.json $top_dir/circuit-lib/circuit-lib.js/package.json.bak
mv $top_dir/circuit-lib/circuit-lib.js/temp.package.json $top_dir/circuit-lib/circuit-lib.js/package.json
(cd $top_dir/circuit-lib/circuit-lib.js && pnpm install --no-frozen-lockfile && pnpm build && pnpm pack)
circuit_lib_tgz=$(ls $top_dir/circuit-lib/circuit-lib.js/*.tgz)

# alter zk.js package.json to use local .tgz files instead of workspace dependencies
generate_temp_package_json $top_dir/zk.js "@lightprotocol/circuit-lib.js" "file:$circuit_lib_tgz" "@lightprotocol/prover.js" "file:$prover_tgz" "@lightprotocol/account.rs" "file:$account_rs_tgz"
mv $top_dir/zk.js/package.json $top_dir/zk.js/package.json.bak
mv $top_dir/zk.js/temp.package.json $top_dir/zk.js/package.json
(cd $top_dir/zk.js && pnpm install --no-frozen-lockfile && pnpm build && pnpm pack)
zkjs_tgz=$(ls $top_dir/zk.js/*.tgz)

# build rpc with altered zk.js
generate_temp_package_json $top_dir/rpc "@lightprotocol/zk.js" "file:$zkjs_tgz"  "@lightprotocol/circuit-lib.js" "file:$circuit_lib_tgz" "@lightprotocol/account.rs" "file:$account_rs_tgz"

mv $top_dir/rpc/package.json $top_dir/rpc/package.json.bak
mv $top_dir/rpc/temp.package.json $top_dir/rpc/package.json

(cd $top_dir/rpc && rm -rf node_modules && npm install)

docker buildx create --name mybuilder
docker buildx use mybuilder
docker run --privileged --rm tonistiigi/binfmt --install all
docker buildx build --platform linux/amd64 -t rpc-app:latest . --load
docker tag rpc-app:latest registry.digitalocean.com/v3-rpc/rpc-app:latest
