#!/usr/bin/env sh

set -eux

root_dir="`git rev-parse --show-toplevel`";
gnark_dir="${root_dir}/gnark-prover"
out_dir="${root_dir}/cli/bin"

if [ ! -e $out_dir ]; then
    mkdir -p $out_dir
fi

# check that the gnark-prover/light-prover executable exists, otherwise build it with `go build`
if [ ! -e "${gnark_dir}/light-prover" ]; then
    cd $gnark_dir
    go get -u golang.org/x/tools/...
    go mod download
    go build
    cd -
fi

cp "${gnark_dir}/light-prover" $out_dir
cp -r "${gnark_dir}/circuits" $out_dir
