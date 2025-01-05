# use with npx nx run @lightprotocol/programs:build
cd account-compression/ && cargo build-sbf && cd -
cd registry/ && cargo build-sbf && cd -
cd system/ && cargo build-sbf && cd -
cd compressed-token/ && cargo build-sbf && cd -