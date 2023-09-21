#!/usr/bin/env sh

source "./scripts/devenv.sh"
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

  if [ "${dir}" = "zk.js" ]; then
    yarn link @lightprotocol/prover.js
  fi

  if [ "${dir}" = "zk.js" ] || [ "${dir}" = "relayer" ]; then
    yarn link @lightprotocol/circuit-lib.js
  fi

  if [ "${dir}" = "circuit-lib/circuit-lib.circom" ] || [ "${dir}" = "cli" ] || [ "${dir}" = "relayer" ] || [ "${dir}" = "system-programs" ]; then
    yarn link @lightprotocol/zk.js
  fi
  yarn install

  if [ "${dir}" != "circuit-lib/circuit-lib.circom" ] ; then
    yarn run build
  fi

  if [ "${dir}" != "circuit-lib/circuit-lib.circom" ] && [ "${dir}" != "relayer" ] && [ "${dir}" != "system-programs" ]; then
      yarn link
  fi
  if [ "${dir}" = "circuit-lib/circuit-lib.circom" ] || [ "${dir}" = "circuit-lib/circuit-lib.js" ]; then
      cd ../..
  else
      cd ..
  fi
}

# need to be built in order because packages depend on each other and need to be linked
build -d "prover.js"
build -d "circuit-lib/circuit-lib.js"
build -d "zk.js"
build -d "system-programs"
build -d "cli"
build -d "relayer"
build -d "circuit-lib/circuit-lib.circom"
