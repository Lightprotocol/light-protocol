#!/bin/bash

# 1. builds zk.js with local circuit-lib and prover.js instead of workspace dependencies
# 2. builds relayer with local zk.js instead of workspace dependency
# 3. builds docker image (consumed in deployRelayer.sh)


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
    echo "Generated temp.package.json for $dir:"
    cat $dir/temp.package.json
}



cd ./circuit-lib/circuit-lib.js && pnpm pack
circuit_lib_tgz=$(ls *.tgz)
cd ../../prover.js && pnpm pack
prover_tgz=$(ls *.tgz)
cd ..

cleanup() {
    echo "Deleting .tgz files..."
    rm -f $(dirname $0)/../zk.js/$zkjs_tgz
    rm -f $(dirname $0)/../circuit-lib/circuit-lib.js/$circuit_lib_tgz
    rm -f $(dirname $0)/../prover.js/$prover_tgz
    echo "Restoring original package.json files..."

    if [ -f $(dirname $0)/../zk.js/package.json.bak ]; then
        rm -f $(dirname $0)/../zk.js/package.json
        mv -f $(dirname $0)/../zk.js/package.json.bak $(dirname $0)/../zk.js/package.json
    fi

    if [ -f $(dirname $0)/../relayer/package.json.bak ]; then
        rm -f $(dirname $0)/../relayer/package.json
        mv -f $(dirname $0)/../relayer/package.json.bak $(dirname $0)/../relayer/package.json
    fi

    echo "Deleting node_modules, cached files, and lock files..."

    rm -rf $(dirname $0)/../zk.js/node_modules
    rm -rf $(dirname $0)/../relayer/node_modules
    rm -f $(dirname $0)/../pnpm-lock.yaml

    echo "Deleting NPM artifacts..."
    rm -rf $(dirname $0)/../zk.js/package-lock.json
    rm -rf $(dirname $0)/../relayer/package-lock.json
    rm -f $(dirname $0)/../pnpm-lock.yaml


    echo "Rebuilding workspace..."
    # bash $(dirname $0)/build.sh

}

trap cleanup EXIT


# alter zk.js package.json to use local .tgz files instead of workspace dependencies
generate_temp_package_json ./zk.js "@lightprotocol/circuit-lib.js" "file:../circuit-lib/circuit-lib.js/$circuit_lib_tgz" "@lightprotocol/prover.js" "file:../prover.js/$prover_tgz"
mv ./zk.js/package.json ./zk.js/package.json.bak
mv ./zk.js/temp.package.json ./zk.js/package.json
cd ./zk.js
pnpm install --no-frozen-lockfile
pnpm build
cd ..

cd ./zk.js && pnpm pack
zkjs_tgz=$(ls *.tgz)
cd ..

# build relayer with altered zk.js
generate_temp_package_json ./relayer "@lightprotocol/zk.js" "file:../zk.js/$zkjs_tgz"  "@lightprotocol/circuit-lib.js" "file:../circuit-lib/circuit-lib.js/$circuit_lib_tgz"

mv ./relayer/package.json ./relayer/package.json.bak
mv ./relayer/temp.package.json ./relayer/package.json

cd ./relayer
# remove node_modules to ensure we're using the local zk.js
rm -rf node_modules
npm install
cd ..


# build docker image
docker buildx create --name mybuilder
docker buildx use mybuilder
docker run --privileged --rm tonistiigi/binfmt --install all
docker buildx build --platform linux/amd64 -t relayer-app:latest . --load
docker tag relayer-app:latest registry.digitalocean.com/v3-relayer/relayer-app:latest
