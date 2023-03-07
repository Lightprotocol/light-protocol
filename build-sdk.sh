cd light-sdk-ts && yarn run build & sleep 5 && kill $! && cd ..;
cd light-circuits && yarn && cd -;
cd light-system-programs && yarn && cd -;
cd mock-app-verifier && yarn && cd -;
