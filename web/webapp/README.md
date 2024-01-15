Reference implementation. Web interface for viewing and managing compressed assets on Solana Devnet.

Supported actions:

- view compressed assets
- view public sol balance
- view tx history
- compress
- decompress
- transfer

### Run

```bash
pnpm run dev
```

Open:

- [ ] using solana wallet adapter
- [ ] compatibility with zk-enabled wallets (account.rs)
- [ ] permissionless SPL/NFTs
- [ ] caching
- [ ] migrate to use releases, localnet

### Debug

```bash
sh testSetupDebug.sh # runs, redis, local test validator, opens a cypress console which can be used to run the test in a browser
```
