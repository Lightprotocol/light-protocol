#!/usr/bin/env bash

keys=(
    "lookup-table"
    "merkle-tree-authority"
    "merkle-tree-set-0"
    "registered-spl-pool-token"
    "registered-spl-pool"
    "registered-sol-pool-token"
    "registered-sol-pool"
    "registered-psp2in2out"
    "registered-psp10in2out"
    "registered-psp4in4out"
    "registered-psp2in2out-storage"
    "authorityJ1RRetZ4ujphU75LP8RadjXMf3sA12yC2R44CF7PmU7i"
    "authorityJ85SuNBBsba7FQS66BiBCQjiQrQTif7v249zL2ffmRZc"
    "authority2cxC8e8uNYLcymH6RTGuJs3N8fXGkwmMpw45pY65Ay86"
    "authority2mNCqdntwtm9cTLjgfdS85JTF92mgNerqA9TgGnxFzLt"
)

values=(
    "DyZnme4h32E66deCvsAV6pVceVw8s6ucRhNcwoofVCem"
    "5EMc8sCbHeb1HtRFifcbCiXN66kX6Wddrd61EkdJun6Y"
    "BrY8P3ZuLWFptfY7qwvkRZkEaD88UEByz9wKRuXFEwhr"
    "2mobV36eNyFGaMTKCHW1Jeoq64tUGuXqA4zGtY8SbxKh"
    "2q4tXrgpsDffibmjfTGHU1gWCjYUfhwFnMyLX6dAhhr4"
    "Eti4Rjkx7ow88XkaFbxRStmwadTp8p9J2nSv7NhtuqDU"
    "EieYsoSQJyr3guR5vDzRMZeQQNA1EymAtp8esEUpjB86"
    "Eo3jtUstuMCvapqXdWiYvoUJS1PJDtKVf6LdsMPdyoNn"
    "9Q5JQPJEqC71R3jTnrnrSEhjMouCVf2dNjURp1L25Wnr"
    "DRwtrkmoUe9VD4T2KRN2A41jqtHgdDeEH8b3sXu7dHVW"
    "9VtiN5ibfgg27WaxJNpbu23VcsETt6LiirPBcsVXjpsc"
    "KitaXMAzb8GPZcc6NW6mE7P6gV2fY3Bp8NqZWfeUwqM"
    "6n2eREPP6bMLLYVJSGcSCULFy7u2WDrx3v5GJR7bByMa"
    "2Qfbp8r5N6omEddWwKG9Cyo52W4VQ69Pk1xDLaW3XJTP"
    "2mNCqdntwtm9cTLjgfdS85JTF92mgNerqA9TgGnxFzLt"
)

export LIGHT_PROTOCOL_CONFIG="`git rev-parse --show-toplevel`/cli/config.json"

# Start test validator.
./cli/test_bin/run test-validator -bs

# Airdrop:
#
# * The main wallet.
# * The Light Protocol admin account.
# * Authority account.
solana airdrop 50000
solana airdrop 50000 $(./cli/test_bin/run config --get | awk '/user/{print $NF;}')
solana airdrop 50000 KitaXMAzb8GPZcc6NW6mE7P6gV2fY3Bp8NqZWfeUwqM

# Initialize:
#
# * Merkle tree authority
# * The 1st Merkle tree set
./cli/test_bin/run merkle-tree-authority:initialize \
    --use-mts-keypair \
    ./target/deploy/merkle_tree_set_0-keypair.json

# Register verifiers.
./cli/test_bin/run verifier:register J1RRetZ4ujphU75LP8RadjXMf3sA12yC2R44CF7PmU7i
./cli/test_bin/run verifier:register J85SuNBBsba7FQS66BiBCQjiQrQTif7v249zL2ffmRZc
./cli/test_bin/run verifier:register 2cxC8e8uNYLcymH6RTGuJs3N8fXGkwmMpw45pY65Ay86
./cli/test_bin/run verifier:register DJpbogMSrK94E1zvvJydtkqoE4sknuzmMRoutd6B7TKj

./cli/test_bin/run pool-type:register 0

./cli/test_bin/run asset-pool:register-sol 0
./cli/test_bin/run asset-pool:register-spl 0 ycrF6Bw3doNPMSDmZM1rxNHimD2bwq1UFmifMCzbjAe

# Dump PDAs into JSON files.
for i in "${!keys[@]}"; do
    key=${keys[$i]}
    value=${values[$i]}
    solana account $value --output-file "cli/accounts/${key}.json" --output "json"
done
