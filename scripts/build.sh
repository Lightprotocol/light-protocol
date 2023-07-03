#!/usr/bin/env bash

set -eux

cleanup_and_install() {
  dir=$1
  build=$2

  pushd $dir

  for sub_dir in node_modules lib bin; do
    if [ -d ./$sub_dir ]; then
      rm -rf $sub_dir
    fi
  done

  yarn install

  if [ "$build" = true ] ; then
    yarn run build
  fi

  popd
}

cleanup_and_install "light-zk.js" true
cleanup_and_install "light-system-programs" false
cleanup_and_install "mock-app-verifier" false
cleanup_and_install "light-circuits" false
cleanup_and_install "relayer" true
