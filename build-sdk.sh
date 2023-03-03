cd light-sdk-ts && npm run build & sleep 5 && kill $! && cd - &&
cd light-circuits && rm -r node_modules && npm i && cd - &&
cd light-system-programs && rm -r node_modules && yarn && cd - &&
cd mock-app-verifier && rm -r node_modules && yarn && cd -;