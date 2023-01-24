cd .. && git clone git@github.com:ananas-block/solana.git && cd solana/validator/ && cargo build && cd -;
cd ligh-protocol-onchain/light-sdk-ts && npm i && npm run build && cd ..;
cd light-system-verifiers && yarn install && anchor build && cd ..;
cd light-circuits && npm i && cd ..;
