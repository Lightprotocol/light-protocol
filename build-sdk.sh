cd light-sdk-ts && yarn run build & sleep 5 && kill $! && cd ..;
cd light-circuits && rm -rf ./node_modules && yarn && cd -;
cd light-system-programs && rm -rf ./node_modules && yarn && cd -;
cd mock-app-verifier && rm -rf ./node_modules && yarn && cd -;
