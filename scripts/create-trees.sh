#!/bin/bash

# Group 1
cargo xtask create-state-tree \
--mt-pubkey ./target/large-v1-state-trees/smtB1XUpt3c7j7udurMdxmAGib7RzCyBXu95fAZoHyT.json \
--nfq-pubkey ./target/large-v1-state-trees/nfqByCmDtLy7pkKpazApswN5H3Y4RSgCVq7NpecLHza.json \
--cpi-pubkey ./target/large-v1-state-trees/cpiBvdvmJCzz9ZLhv3cEih3V8u52z2CzCoWRyGGHqN4.json \
--index 11 --network local

# Group 2
cargo xtask create-state-tree \
--mt-pubkey ./target/large-v1-state-trees/smtCg6rdiVANNqgZtBUzSuR5ZCcCmutiBM1WF82dA5V.json \
--nfq-pubkey ./target/large-v1-state-trees/nfqCyWDJhvnCchFxZyTqMMirWhnQTLUzbFdSEHSxLH9.json \
--cpi-pubkey ./target/large-v1-state-trees/cpiCooH1oq5T7qvSgKuAPHAYJZfWBNYfARjcaJ4akgL.json \
--index 12 --network local

# Group 3
cargo xtask create-state-tree \
--mt-pubkey ./target/large-v1-state-trees/smtd4RMDUcdvvfnjYMq3HzyyqTmgojMHAYrKd5oSHGa.json \
--nfq-pubkey ./target/large-v1-state-trees/nfqDgCgnkyYmDav7SCT41MHLqBVDw7ZMZ9g3FUAhKA5.json \
--cpi-pubkey ./target/large-v1-state-trees/cpiD1tk8EYc3CvKqUc58Pzw9XWHDWpNVU8TnCFFExDs.json \
--index 13 --network local

# Group 4
cargo xtask create-state-tree \
--mt-pubkey ./target/large-v1-state-trees/smtEC1YEbkASxidPBqCvv4ZnHpiGbEoTR6jxMorukfw.json \
--nfq-pubkey ./target/large-v1-state-trees/nfqEqgUCSzv46UsHVCKCS4xqVpmgJGP5TbFeeNcRVTT.json \
--cpi-pubkey ./target/large-v1-state-trees/cpiEs1xm1T5XzXAdv7Ed6vMqFTPvatv45SHjHCF9psa.json \
--index 14 --network local

# Group 5
cargo xtask create-state-tree \
--mt-pubkey ./target/large-v1-state-trees/smtFhRnMUAzVPvK3hpqW8bdZ57EGecBZ2amgTSHDfvh.json \
--nfq-pubkey ./target/large-v1-state-trees/nfqFogABA4EtEauP8ti3KA96qMv6QBoGda9cTAcfKph.json \
--cpi-pubkey ./target/large-v1-state-trees/cpiFgzewJNcNZWzb5MGxrhMigB86VC34hP1XpXYLoMG.json \
--index 15 --network local
