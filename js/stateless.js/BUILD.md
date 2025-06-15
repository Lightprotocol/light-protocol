Stateless.js compiles with V1 API by default. To switch over to V2 endpoints (with backward compatibility for V1 state), run:

```bash
pnpm build:v2
# or
LIGHT_PROTOCOL_VERSION=V2 pnpm build
```

## Usage in Code

```typescript
// From rpc.ts
const endpoint = featureFlags.isV2()
    ? versionedEndpoint('getCompressedAccountV2')
    : versionedEndpoint('getCompressedAccount');
```
