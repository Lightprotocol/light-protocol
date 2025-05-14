This project demonstrates how to use `@lightprotocol/stateless.js` to interact
with the ZK Compression API in a browser environment.

0. Build the Monorepo.

```bash
cd ../../../ &&
. ./scripts/devenv.sh &&
./scripts/install.sh &&
./scripts/build.sh
```

1. Start a light test-validator using the CLI

```bash
cd cli &&
light test-validator
```

2. Start the app

```bash
cd ../examples/browser/nextjs &&
pnpm dev
```

This will serve and mount the app at http://localhost:1234 and run the code
defined in `page.tsx`.
