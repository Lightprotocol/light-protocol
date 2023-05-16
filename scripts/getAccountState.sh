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
    ["registeredVerifierPda_1"]="CqUS5VyuGscwLMTbfUSAA1grmJYzDAkSR39zpbwW2oV5"
    ["registeredVerifierPda_2"]="7RCgKAJkaR4Qsgve8D7Q3MrVt8nVY5wdKsmTYVswtJWn"
    ["registeredVerifierPda_3"]="9VtiN5ibfgg27WaxJNpbu23VcsETt6LiirPBcsVXjpsc"
    ["authorityJ1RRetZ4ujphU75LP8RadjXMf3sA12yC2R44CF7PmU7i"]="KitaXMAzb8GPZcc6NW6mE7P6gV2fY3Bp8NqZWfeUwqM"
    ["authority3KS2k14CmtnuVv2fvYcvdrNgC94Y11WETBpMUGgXyWZL"]="EjGpk73m5KxndbUVXcoT3UQsPLp5eK4h1H8kXVHEbf3f"
    ["authorityGFDwN8PXuKZG2d2JLxRhbggXYe9eQHoGYoYK5K3G5tV8"]="GF5uKvLBvgZk27iJ5uSHNoNUhsQ7c55r9EVjTGGZ5s8W"
    ["authorityDJpbogMSrK94E1zvvJydtkqoE4sknuzmMRoutd6B7TKj"]="2mNCqdntwtm9cTLjgfdS85JTF92mgNerqA9TgGnxFzLt"
)

for key in "${!keys[@]}"; do
    value=${keys[$key]}
    solana account $value --output-file "accounts/${key}.json" --output "json"
done
