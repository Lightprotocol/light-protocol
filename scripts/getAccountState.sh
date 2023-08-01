#!/usr/bin/env sh

keys=(
    "LOOK_UP_TABLE"
    "merkleTreeAuthorityPda"
    "eventMerkleTreePubkey"
    "transactionMerkleTreePubkey"
    "registered spl pool token pda"
    "registered spl pool pda"
    "REGISTERED_POOL_PDA_SOL"
    "registered sol pool"
    "registeredVerifierPda"
    "registeredVerifierPda_1"
    "registeredVerifierPda_2"
    "registeredVerifierPda_3"
    "authorityJ1RRetZ4ujphU75LP8RadjXMf3sA12yC2R44CF7PmU7i"
    "authorityJ85SuNBBsba7FQS66BiBCQjiQrQTif7v249zL2ffmRZc"
    "authority2cxC8e8uNYLcymH6RTGuJs3N8fXGkwmMpw45pY65Ay86"
    "authority2mNCqdntwtm9cTLjgfdS85JTF92mgNerqA9TgGnxFzLt"
)

values=(
    "DyZnme4h32E66deCvsAV6pVceVw8s6ucRhNcwoofVCem"
    "5EMc8sCbHeb1HtRFifcbCiXN66kX6Wddrd61EkdJun6Y"
    "6x8FxrUqokbXCvnPT84Qvi5QcXVdQNv74Z5ZmS6znWAc"
    "DDx9XekF4emf7p7QyUYcCqZcPJtmzUYmYir54tQBbVBv"
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

for i in "${!keys[@]}"; do
    key=${keys[$i]}
    value=${values[$i]}
    solana account $value --output-file "test-env/accounts/${key}.json" --output "json"
done
