#!/usr/bin/env sh

set -eux

cleanup_and_install() {
  if [ "${#}" -ne 6 ]; then
    echo "Usage: cleanup_and_install --dir <dir> --yarn <yarn_build> --anchor <anchor_build>"
    exit 1
  fi
  while [ "${#}" -gt 0 ]; do
    case "${1}" in
      -d|--dir)
        dir="${2}"
        shift 2
        ;;
      -y|--yarn)
        yarn="${2}"
        shift 2
        ;;
      -a|--anchor)
        anchor="${2}"
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

  if "${dir}" "${dir}" == "light-zk.js" ]]; then
        yarn link @lightprotocol/prover.js
  fi

  if [ "${dir}" = "light-circuits" ] || [ "${dir}" = "cli" ] || [ "${dir}" = "relayer" ] || [ "${dir}" = "light-system-programs" ]; then
      yarn link @lightprotocol/zk.js
  fi
  yarn install

  if [ "${yarn}" = true ] ; then
    yarn run build
  fi

  if [ "${anchor}" = true ] ; then
    light-anchor build
  fi
  

  
  if [ "${dir}" != "light-system-programs" ]; then
      yarn link
  fi

  cd ..
}

cleanup_and_install --dir "light-prover.js" --yarn true --anchor false
cleanup_and_install --dir "light-zk.js" --yarn true --anchor false
cleanup_and_install -d "light-system-programs" -y true -a false
cleanup_and_install -d "cli" -y true -a false
cleanup_and_install -d "light-circuits" -y false -a false
cleanup_and_install -d "relayer" -y true -a false
