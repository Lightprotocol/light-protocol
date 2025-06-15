To debug CI issues with the CLI locally, run:

```bash
# For V1 tests (matches cli-v1.yml workflow)
./debug-ci-locally.sh V1

# For V2 tests (matches cli-v2.yml workflow)
./debug-ci-locally.sh V2
```

### Debugging

If the prover fails, check:

```bash
cat test-ledger/prover-*.log

redis-cli ping
```

### Matching CI Exactly

To match CI as closely as possible:

- Clear all caches before running
- Use the same Node version as CI
- Run with fresh dependencies: `rm -rf node_modules && pnpm install`
- Disable any local config that might interfere

Both workflows:

1. Start Redis service on port 6379
2. Set REDIS_URL environment variable
3. Build dependencies with specific protocol version
4. Run CLI tests
5. Display prover logs on failure
