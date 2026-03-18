<p align="center">
  <img src="https://github.com/ldiego08/light-protocol/raw/main/assets/logo.svg" width="90" />
</p>

<h1 align="center">@lightprotocol/token-kit</h1>

<p align="center">
  <b>TypeScript SDK for Light Protocol compressed tokens, built on Solana Kit (web3.js v2).</b>
</p>

### Installation

```bash
pnpm add @lightprotocol/token-kit @solana/kit
```

Requires `@solana/kit ^2.1.0` as a peer dependency.

### Usage

```typescript
import {
  createLightIndexer,
  buildCompressedTransfer,
} from '@lightprotocol/token-kit';

// Connect to Photon indexer
const indexer = createLightIndexer('https://photon.helius.dev');

// Build a compressed token transfer
const result = await buildCompressedTransfer(indexer, {
  owner: ownerAddress,
  mint: mintAddress,
  amount: 1000n,
  recipientOwner: recipientAddress,
  feePayer: payerAddress,
});

// result.instruction — Transfer2 instruction ready for a transaction
```

### Compress and decompress

```typescript
import { buildCompress, buildDecompress } from '@lightprotocol/token-kit';

// Compress: SPL token account → compressed token accounts
const compressIx = await buildCompress({
  rpc,
  source: splAta,
  owner: ownerAddress,
  mint: mintAddress,
  amount: 1000n,
  decimals: 9,
  tokenProgram: SPL_TOKEN_PROGRAM_ID,
  outputQueue: queueAddress,
});

// Decompress: compressed → SPL token account
const { instruction: decompressIx } = await buildDecompress({
  rpc,
  indexer,
  owner: ownerAddress,
  mint: mintAddress,
  amount: 1000n,
  destination: splAta,
  decimals: 9,
  tokenProgram: SPL_TOKEN_PROGRAM_ID,
});
```

### Wrap and unwrap (SPL ↔ Light Token)

```typescript
import { buildWrap, buildUnwrap } from '@lightprotocol/token-kit';

// Wrap: SPL associated token account → Light Token associated token account
const wrapIx = await buildWrap({
  rpc,
  source: splAta,
  destination: lightTokenAta,
  owner: ownerAddress,
  mint: mintAddress,
  amount: 1000n,
  decimals: 9,
  tokenProgram: SPL_TOKEN_PROGRAM_ID,
});

// Unwrap: Light Token → SPL
const unwrapIx = await buildUnwrap({
  rpc,
  source: lightTokenAta,
  destination: splAta,
  owner: ownerAddress,
  mint: mintAddress,
  amount: 1000n,
  decimals: 9,
  tokenProgram: SPL_TOKEN_PROGRAM_ID,
});
```

### Mint management

```typescript
import {
  buildCreateMint,
  buildDecompressMint,
  buildMintToCompressed,
  buildUpdateMetadataField,
} from '@lightprotocol/token-kit';

// Create a compressed mint with metadata
const createMintIx = await buildCreateMint({
  mintSigner,
  authority: authorityAddress,
  feePayer: payerAddress,
  outOutputQueue: queueAddress,
  merkleTree: treeAddress,
  decimals: 9,
  mintAuthorityBytes: authorityBytes,
  extensions: [{ type: 'TokenMetadata', data: metadata }],
  proof: addressProof,
});

// Mint to compressed accounts
const mintIx = buildMintToCompressed({
  authority: authorityAddress,
  feePayer: payerAddress,
  mintSigner,
  outOutputQueue: queueAddress,
  merkleTree: treeAddress,
  leafIndex: 0,
  rootIndex: 0,
  recipients: [{ recipient: recipientBytes, amount: 1000n }],
});
```

### Query functions

```typescript
import { getAtaInterface, getMintInterface } from '@lightprotocol/token-kit';

// Unified balance view (hot + cold + SPL)
const account = await getAtaInterface(rpc, indexer, owner, mint, hotAta, splAta);
console.log(account.totalBalance); // hot + cold + SPL

// Mint info
const mintInfo = await getMintInterface(rpc, mintAddress);
console.log(mintInfo.decimals, mintInfo.supply);
```

### What's included

**Instruction builders** (low-level)

| Builder | Description |
|---------|-------------|
| `createTransferInstruction` | Transfer between Light Token accounts |
| `createTransfer2Instruction` | Batch transfer with compress/decompress |
| `createMintToInstruction` | Mint tokens to Light Token account |
| `createBurnInstruction` | Burn tokens |
| `createApproveInstruction` | Approve delegate |
| `createFreezeInstruction` / `createThawInstruction` | Freeze/thaw accounts |
| `createAssociatedTokenAccountInstruction` | Create Light Token associated token account |
| `createTokenAccountInstruction` | Create token account with extensions |
| `createCloseAccountInstruction` | Close zero-balance account |
| `createWrapInstruction` / `createUnwrapInstruction` | SPL ↔ Light Token |
| `createMintActionInstruction` | Batch mint operations |
| `createClaimInstruction` | Claim rent from compressible accounts |
| `createWithdrawFundingPoolInstruction` | Withdraw from funding pool |
| `createSplInterfaceInstruction` | Register SPL interface PDA |
| `addSplInterfacesInstruction` | Add additional pool PDAs |

**High-level builders** (load + select + proof + instruction)

| Builder | Description |
|---------|-------------|
| `buildCompressedTransfer` | Compressed-to-compressed transfer |
| `buildTransferDelegated` | Transfer via delegate authority |
| `buildTransferInterface` | Auto-routing transfer |
| `buildCompress` | SPL → compressed accounts |
| `buildDecompress` | Compressed → SPL account |
| `buildCompressSplTokenAccount` | Compress SPL token account |
| `buildWrap` / `buildUnwrap` | SPL ↔ Light Token |
| `buildCreateMint` | Create compressed mint |
| `buildDecompressMint` | Decompress mint to on-chain |
| `buildUpdateMintAuthority` | Update mint authority |
| `buildUpdateFreezeAuthority` | Update freeze authority |
| `buildUpdateMetadataField` | Update metadata name/symbol/uri |
| `buildUpdateMetadataAuthority` | Update metadata authority |
| `buildRemoveMetadataKey` | Remove metadata key |
| `buildMintToCompressed` | Mint to compressed accounts |
| `buildMintToInterface` | Mint to Light Token account |
| `buildApproveAndMintTo` | Approve delegate + mint |
| `buildLoadAta` | Load cold balance to hot |

**Query functions**

| Function | Description |
|----------|-------------|
| `getAtaInterface` | Unified balance view (hot + cold + SPL) |
| `getMintInterface` | On-chain mint info |

**Indexer client**

| Method | Description |
|--------|-------------|
| `getCompressedTokenAccountsByOwner` | Fetch compressed accounts |
| `getValidityProof` | Fetch validity proof |
| `getCompressedTokenBalancesByOwner` | Balances grouped by mint |
| `getCompressedMintTokenHolders` | Token holders for a mint |
| `getCompressedTokenAccountBalance` | Single account balance |
| `getSignaturesForTokenOwner` | Transaction signatures |

**Utilities**

- PDA derivation: `deriveAssociatedTokenAddress`, `deriveMintAddress`, `derivePoolAddress`
- SPL interface: `getSplInterfaceInfo`, `getSplInterfaceInfos`, `selectSplInterfaceInfo`, `deriveSplInterfaceInfo`
- Account loading: `loadTokenAccountsForTransfer`, `selectAccountsForAmount`, `loadAllTokenAccounts`
- Validation: `isLightTokenAccount`, `determineTransferType`, `validateAtaDerivation`
- Codecs: Borsh-compatible encoders/decoders for all instruction data types

### Feature parity with @lightprotocol/compressed-token

| Feature | compressed-token | token-kit |
|---------|-----------------|-----------|
| Compressed transfer | `transfer` | `buildCompressedTransfer` |
| Delegated transfer | `transferDelegated` | `buildTransferDelegated` |
| Transfer interface | `transferInterface` | `buildTransferInterface` |
| Compress SPL→compressed | `compress` | `buildCompress` |
| Decompress compressed→SPL | `decompress` | `buildDecompress` |
| Compress SPL account | `compressSplTokenAccount` | `buildCompressSplTokenAccount` |
| Wrap SPL→Light Token | — | `buildWrap` |
| Unwrap Light Token→SPL | — | `buildUnwrap` |
| Create mint | `createMintInterface` | `buildCreateMint` |
| Decompress mint | `decompressMint` | `buildDecompressMint` |
| Update mint authority | `updateMintAuthority` | `buildUpdateMintAuthority` |
| Update freeze authority | `updateFreezeAuthority` | `buildUpdateFreezeAuthority` |
| Update metadata | `updateMetadataField` | `buildUpdateMetadataField` |
| Update metadata authority | `updateMetadataAuthority` | `buildUpdateMetadataAuthority` |
| Remove metadata key | `removeMetadataKey` | `buildRemoveMetadataKey` |
| Mint to compressed | `mintToCompressed` | `buildMintToCompressed` |
| Mint to interface | `mintToInterface` | `buildMintToInterface` |
| Approve + mint | `approveAndMintTo` | `buildApproveAndMintTo` |
| Load ATA | `loadAta` | `buildLoadAta` |
| Create SPL interface | `createSplInterface` | `createSplInterfaceInstruction` |
| Add SPL interfaces | `addSplInterfaces` | `addSplInterfacesInstruction` |
| Account interface | `getAtaInterface` | `getAtaInterface` |
| Mint interface | `getMintInterface` | `getMintInterface` |
| Token balances by owner | `getCompressedTokenBalancesByOwner` | `getCompressedTokenBalancesByOwner` |
| Mint token holders | `getCompressedMintTokenHolders` | `getCompressedMintTokenHolders` |

### Documentation and examples

- [ZK Compression docs](https://www.zkcompression.com)
- [Compressed Token guides](https://www.zkcompression.com/compressed-tokens/guides)
- [Source code](https://github.com/lightprotocol/light-protocol/tree/main/js/token-kit)

### Getting help

Check out the [Light](https://discord.gg/CYvjBgzRFP) and [Helius](https://discord.gg/Uzzf6a7zKr) Developer Discord servers.

### License

Apache-2.0
