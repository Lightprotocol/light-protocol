cd light-sdk-ts && npm run build & sleep 5 && kill $! && cd ..  ;
cd light-circuits && rm node_modules/light-sdk/ -r && npm i && cd -;
cd light-system-programs && rm node_modules/light-sdk/ -r && npm i && cd -;
cd mock-app-verifier && rm node_modules/light-sdk/ -r && npm i && cd -;
