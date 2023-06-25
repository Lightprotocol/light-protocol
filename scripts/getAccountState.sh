#!/bin/bash

#!/bin/bash

declare -A keys=(
    ["LOOK_UP_TABLE"]="DyZnme4h32E66deCvsAV6pVceVw8s6ucRhNcwoofVCem"
    ["merkleTreeAuthorityPda"]="5EMc8sCbHeb1HtRFifcbCiXN66kX6Wddrd61EkdJun6Y"
    ["messageMerkleTreePubkey"]="65ugKwDCTmQvEAsEep842CjZArCmUQ2r37RW9VDLbNKy"
    ["transactionMerkleTreePubkey"]="DCxUdYgqjE6AR9m13VvqpkxJqGJYnk8jn8NEeD3QY3BU"
    ["registered spl pool token pda"]="2mobV36eNyFGaMTKCHW1Jeoq64tUGuXqA4zGtY8SbxKh"
    ["registered spl pool pda"]="2q4tXrgpsDffibmjfTGHU1gWCjYUfhwFnMyLX6dAhhr4"
    ["REGISTERED_POOL_PDA_SOL"]="Eti4Rjkx7ow88XkaFbxRStmwadTp8p9J2nSv7NhtuqDU"
    ["registered sol pool"]="EieYsoSQJyr3guR5vDzRMZeQQNA1EymAtp8esEUpjB86"
    ["registeredVerifierPda"]="Eo3jtUstuMCvapqXdWiYvoUJS1PJDtKVf6LdsMPdyoNn"
    ["registeredVerifierPda_1"]="9Q5JQPJEqC71R3jTnrnrSEhjMouCVf2dNjURp1L25Wnr"
    ["registeredVerifierPda_2"]="DRwtrkmoUe9VD4T2KRN2A41jqtHgdDeEH8b3sXu7dHVW"
    ["registeredVerifierPda_3"]="9VtiN5ibfgg27WaxJNpbu23VcsETt6LiirPBcsVXjpsc"
    ["authorityJ1RRetZ4ujphU75LP8RadjXMf3sA12yC2R44CF7PmU7i"]="KitaXMAzb8GPZcc6NW6mE7P6gV2fY3Bp8NqZWfeUwqM"
    ["authorityJ85SuNBBsba7FQS66BiBCQjiQrQTif7v249zL2ffmRZc"]="6n2eREPP6bMLLYVJSGcSCULFy7u2WDrx3v5GJR7bByMa"
    ["authority2cxC8e8uNYLcymH6RTGuJs3N8fXGkwmMpw45pY65Ay86"]="2Qfbp8r5N6omEddWwKG9Cyo52W4VQ69Pk1xDLaW3XJTP"
    ["authority2mNCqdntwtm9cTLjgfdS85JTF92mgNerqA9TgGnxFzLt"]="2mNCqdntwtm9cTLjgfdS85JTF92mgNerqA9TgGnxFzLt"
)

for key in "${!keys[@]}"; do
    value=${keys[$key]}
    solana account $value --output-file "accounts/${key}.json" --output "json"
done
