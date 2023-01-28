cd ..;
if [ ! -f ./solana ]; then
    git clone git@github.com:ananas-block/solana.git;
    cd solana && git fetch -a &&  git checkout audit &&  cd validator/ && cargo build && cd ../..;
fi
cd light-protocol-onchain/light-sdk-ts && npm i && npm run build; \
cd ../light-system-programs && yarn install && anchor build; \
cd ../light-circuits && npm i && cd ..;

