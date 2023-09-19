#!/usr/bin/env sh

set -eux

build() {
  if [ "${#}" -ne 2 ]; then
    echo "Usage: build --dir <dir>"
    exit 1
  fi
  while [ "${#}" -gt 0 ]; do
    case "${1}" in
      -d|--dir)
        dir="${2}"
        shift 2
        ;;
      *)
        echo "Unknown option: ${1}"
        return 1
        ;;
    esac
  done

  cd "${dir}"

  for sub_dir in node_modules lib bin; do
    if [ -d "./${sub_dir}" ]; then
      rm -rf "${sub_dir}"
    fi
  done

  if [ "${dir}" = "light-zk.js" ]; then
    yarn link @lightprotocol/prover.js
    yarn link @lightprotocol/circuit-lib.js
  fi

  if [ "${dir}" = "circuit-lib/circuit-lib.circom" ]; then
    yarn link @lightprotocol/cli
  fi

  if [ "${dir}" = "circuit-lib/circuit-lib.circom" ] ||  "${dir}" = "circuit-lib/circuit-lib.js" ] || [ "${dir}" = "cli" ] || [ "${dir}" = "relayer" ] || [ "${dir}" = "light-system-programs" ]; then
    yarn link @lightprotocol/zk.js
  fi
  yarn install

  if [ "${dir}" != "light-circuits" ] ; then
    yarn run build
  fi

  if [ "${dir}" != "circuit-lib/circuit-lib.circom" ] && [ "${dir}" != "relayer" ] && [ "${dir}" != "light-system-programs" ]; then
      yarn link
  fi

  cd -
}

# need to be built in order because packages depend on each other and need to be linked
build -d "light-prover.js"
build -d "circuit-lib/circuit-lib.js"
build -d "light-zk.js"
build -d "circuit-lib/circuit-lib.circom"
build -d "light-system-programs"
build -d "cli"
build -d "relayer"
