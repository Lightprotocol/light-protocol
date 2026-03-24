## [0.23.0] - 2026-03-24

Stable release. It supersedes the **`0.23.0-beta.x`** line and is intended for production use with **ZK Compression V2**. Upgrade from **`0.22.x`** or any beta version requires addressing the breaking changes listed below.

### Highlights

- **V2 is the default for npm consumers:** **`@lightprotocol/compressed-token@0.23.0`** pairs with **`@lightprotocol/stateless.js@0.23.0`**. You do **not** need **`LIGHT_PROTOCOL_VERSION=V2`** in app code for published packages. Set **`LIGHT_PROTOCOL_VERSION=V1`** only when **building these packages from source** and you need V1 artifacts.
- **Light token surface:** Public names are **LightToken** / **`LIGHT_TOKEN_PROGRAM_ID`** (replacing CToken / **`CTOKEN_PROGRAM_ID`**).
- **Interface transfers & loads:** The new token API is much closer to spl-token now, and uses the same Account types. Changes include: Recipient wallet vs explicit destination account, **owner pubkey + signing authority** split for delegates, batched **`TransactionInstruction[][]`**, and **`loadAta`** / **`createLoadAtaInstructions`** replacing **`decompressInterface`**.

### Breaking changes (since 0.22.0)

**Transfers (actions)**

- **`transferInterface`**: `recipient` is the **recipient wallet** `PublicKey` (not a pre-derived ATA). Signature:

    `transferInterface(rpc, payer, source, mint, recipient, owner, authority, amount, programId?, confirmOptions?, options?, decimals?)`
    - **`owner`**: source token-account **owner** pubkey (for ATA derivation / source checks).
    - **`authority`**: **`Signer`** — use **`owner`** for owner-signed flows (`owner.publicKey` + `owner`), or **`delegate`** for delegated flows (`owner.publicKey` + `delegate`).

- **`transferToAccountInterface`**: explicit **`destination`** token account; same **`owner` / `authority`** split:

    `transferToAccountInterface(rpc, payer, source, mint, destination, owner, authority, amount, programId?, confirmOptions?, options?, decimals?)`

- **Multiple transactions:** for large cold-input fan-in, these actions may send **parallel load transactions** followed by a **final transfer** (Solana size / proof limits). Instruction builders expose the same as **`TransactionInstruction[][]`**.

**Transfers (instruction builders)**

- **`createTransferInterfaceInstructions`**: returns **`TransactionInstruction[][]`** (one inner array per transaction). Wallet recipient; embeds idempotent recipient ATA creation in the final batch.

    `createTransferInterfaceInstructions(rpc, payer, mint, amount, owner, recipient, decimals, options?, programId?)`

- **`createTransferToAccountInterfaceInstructions`**: same batching shape; **`destination`** is the token account address.

    `createTransferToAccountInterfaceInstructions(rpc, payer, mint, amount, owner, destination, decimals, options?, programId?)`

- **`InterfaceOptions`**: **`wrap`** and related options are nested here; **`programId`** is **not** inside `InterfaceOptions` (flat parameter, SPL-style). **`InterfaceOptions.owner` was removed** — use flat **`owner`** args and, for delegated **planning**, **`options.delegatePubkey`**.

- **`decimals`** is required on these v3 builders unless you use a higher-level wrapper that resolves decimals for you.

**Load / decompress**

- **`decompressInterface`** removed. Use **`loadAta`** (action) or **`createLoadAtaInstructions`** (builder). **`createLoadAtaInstructions`** returns **`TransactionInstruction[][]`**, not a flat instruction array.

- **Delegated loads:** flat args still pass the token-account **`owner`** pubkey; when the **`Signer`** is a **delegate**, set **`interfaceOptions.delegatePubkey`** to the delegate’s pubkey (do not stuff the owner into removed **`InterfaceOptions.owner`**).

- **`createLoadAtaInstructionsFromInterface`** is **not** exported from the package root; use **`createLoadAtaInstructions`**.

**Renames (CToken → LightToken)**

- Examples: **`CTOKEN_PROGRAM_ID` → `LIGHT_TOKEN_PROGRAM_ID`**, **`createAssociatedCTokenAccountInstruction` → `createAssociatedLightTokenAccountInstruction`**, **`parseCTokenHot` / `parseCTokenCold` → `parseLightTokenHot` / `parseLightTokenCold`**, **`mintToCToken` → `mintToLightToken`**, **`createCTokenTransferInstruction` → `createLightTokenTransferInstruction`**.

**`createLightTokenTransferInstruction`**

- Instruction **data layout** (9-byte) and **account metas** changed: includes **`system_program`** and **`fee_payer`**; **`owner` is writable** (compressible rent top-ups).

**`createDecompressInterfaceInstruction`**

- **`decimals: number`** required after **`splInterfaceInfo`** where applicable for SPL destination decompression.

**Deprecated dispatcher**

- **`createTransferInterfaceInstruction`** (monolithic multi-program helper): deprecated; use **`createLightTokenTransferInstruction`** or SPL **`createTransferCheckedInstruction`** as appropriate.

**Token pool / SPL interface**

- Prefer **`CompressedTokenProgram.createSplInterface`**; **`createTokenPool`** is deprecated.

**Removed exports**

- **`createLoadAccountsParams`**, **`calculateCompressibleLoadComputeUnits`**, and related types — use **`createLoadAtaInstructions`** and **`calculateLoadBatchComputeUnits`**.

**`mintToCompressed`**

- **`maxTopUp`** was inserted **before** optional **`confirmOptions`** — update positional call sites.

**RPC / typing strictness**

- **`getAccountInterface`**, **`getAtaInterface`**, **`getMintInterface`**: unexpected failures and wrong-program owners surface as **errors** instead of being treated like empty/not-found in several cases.

**Synthetic delegate views**

- In **`getAtaInterface` / `getAccountInterface`**, canonical delegate and **`delegatedAmount`** aggregation follow updated rules (hot delegate preferred; cold delegates aligned with hot).

### New APIs (consumer-visible)

- **`approveInterface`**, **`revokeInterface`**; builders **`createApproveInterfaceInstructions`**, **`createRevokeInterfaceInstructions`**; **`createLightTokenApproveInstruction`**, **`createLightTokenRevokeInstruction`**.
- **`createUnwrapInstructions`**, **`selectInputsForAmount`**.
- **`createLightTokenFreezeAccountInstruction`**, **`createLightTokenThawAccountInstruction`**.
- **`MAX_TOP_UP`** and **`maxTopUp`** on compressible instruction builders and selected actions.
- **`sliceLast`**, **`chunkAccountsByTreeVersion`**, **`assertUniqueInputHashes`** for batch orchestration.
- **`assertV2Only`** on load/decompress paths — V1 compressed inputs fail fast with a clear error.
- **`LightTokenProgram`** alias for **`CompressedTokenProgram`**.
- **`getOrCreateAtaInterface`** / create-mint flows: decompressed mint handling integrated where documented so typical flows do not require a separate **`decompressMint`** before ATAs.

### Changed / fixed (summary)

- **`approveInterface` / `revokeInterface`**: optional **`InterfaceOptions`** (incl. **`wrap`**) and trailing **`decimals`**; unified approve/revoke entrypoints match the same options pattern.

- **`@lightprotocol/compressed-token/unified`**: transfer-related entrypoints keep **`wrap: true`**; use the non-unified exports for explicit **`programId`** / no-wrap SPL Token-2022 style flows.

- **`loadAta`**: may send **parallel** load batches; frozen-source handling and SPL interface error propagation improved.
- **`maxTopUp`** default **`MAX_TOP_UP` (65535)** so rent top-ups are not silently blocked.
- **Browser bundles:** minified output keeps **`AccountMeta`** **`isSigner` / `isWritable`** as real **booleans** (Terser; see **`@lightprotocol/stateless.js`** changelog).
- **`delegatedAmount`** parsed from CompressedOnly TLV where applicable.

---

## [0.23.0-beta.11]

### Added

- **Delegate approval and revocation** for SPL Token, Token-2022, and light-token, aligned with existing interface helpers:
    - **Actions:** `approveInterface`, `revokeInterface`.
    - **Instruction builders:** `createApproveInterfaceInstructions`, `createRevokeInterfaceInstructions` — each inner array is one transaction’s instructions (same batching style as other interface instruction builders).
    - **Program-level helpers:** `createLightTokenApproveInstruction`, `createLightTokenRevokeInstruction`
- **Shared options:** approve/revoke accept optional `InterfaceOptions` (same type as `transferInterface`), including `splInterfaceInfos` when you need to supply SPL interface pool accounts explicitly.

### Changed

- **`approveInterface` / `revokeInterface`:** optional `options?: InterfaceOptions` and `decimals?: number` after `wrap`. For SPL or Token-2022 with `wrap: false`, the SDK skips an extra mint fetch used only for decimals on that path (you can still pass `decimals` when your flow requires it).
- **`@lightprotocol/compressed-token/unified`:** approve/revoke APIs accept the same optional `options` and `decimals`; unified entrypoints keep their existing default wrapping behavior (`wrap: true`).
- **Interface API normalization:** `programId` is now flat on transfer interface helpers/instruction builders (SPL-style), while `wrap` is consistently nested under `InterfaceOptions` across transfer/approve/revoke/load interface methods and their unified/root wrappers.

### Breaking Changes

- **Transfer interface owner/authority split:** `transferInterface`, `transferToAccountInterface`, and the unified wrappers now take the token-account owner pubkey and the signing authority separately.
    - **Action:** `transferInterface(rpc, payer, source, mint, recipient, owner, authority, amount, ...)`
    - **Action:** `transferToAccountInterface(rpc, payer, source, mint, destination, owner, authority, amount, ...)`
    - Owner-signed flows now pass `owner.publicKey, owner`; delegated flows pass `ownerPublicKey, delegateSigner`.
- **`InterfaceOptions.owner` removed:** transfer interface helpers no longer accept the account owner inside `InterfaceOptions`.
    - Instruction builders keep flat `owner` as the canonical account owner.
    - Delegated instruction planning must use `options.delegatePubkey`.

### Fixed

- **Browser bundles:** Terser no longer rewrites booleans to integers in minified output, keeping `AccountMeta` flags compatible with `@solana/web3.js` and runtime expectations (same change as `stateless.js`; see [#2347](https://github.com/Lightprotocol/light-protocol/pull/2347)).

## [0.23.0-beta.10]

### Breaking Changes

- **`transferInterface` and `createTransferInterfaceInstructions`** now take a recipient wallet address and ensure recipient ATA internally.
    - **Action:** `transferInterface(rpc, payer, source, mint, recipient, owner, authority, amount, ...)` — `recipient` is the wallet public key.
    - **Instruction builder:** `createTransferInterfaceInstructions(rpc, payer, mint, amount, sender, recipient, decimals, options?)` — derives destination ATA and inserts idempotent ATA-create internally.
    - **Advanced explicit-account path:** use `transferToAccountInterface(...)` and `createTransferToAccountInterfaceInstructions(...)` for destination token-account routing (program-owned/custom accounts), preserving the previous destination-account behavior.
    - **`decimals` is required** on v3 action-level instruction builders. Fetch with `getMintInterface(rpc, mint).mint.decimals` if not already threaded.

- **Root export removed:** `createLoadAtaInstructionsFromInterface` is no longer exported from the package root. Use `createLoadAtaInstructions` (public API) and pass ATA/owner/mint directly.

- **Interface RPC error semantics are now strict (no silent downgrade to not-found).**
    - `getAccountInterface` / `getAtaInterface`: unexpected RPC failures (on-chain fetch, compressed fetch, parsing) now throw immediately instead of being silently ignored when another source produced data.
    - `getAccountInterface` light-token mode and SPL/T22 mode now surface `TokenInvalidAccountOwnerError` when an on-chain account exists at the queried address but is owned by a different program.
    - `getMintInterface` (decompressed light-mint branch) now validates the on-chain mint owner and throws `TokenInvalidAccountOwnerError` on mismatch; it also forwards the provided `commitment` to on-chain `getAccountInfo`.
    - **Migration impact:** callers that previously interpreted these paths as empty/not-found must now handle thrown errors explicitly (retry/backoff or surfacing RPC health).

- **`decompressInterface` removed.** Use `loadAta` (action) or `createLoadAtaInstructions` (instruction builder) instead. `decompressInterface` did not support >8 compressed inputs and has been fully removed.
    - **Action (send transaction):** Replace `decompressInterface(rpc, payer, owner, mint, amount?, destinationAta?, destinationOwner?, splInterfaceInfo?, confirmOptions?)` with `loadAta(rpc, ata, owner, mint, payer?, confirmOptions?, interfaceOptions?, wrap?)`. Derive the target ATA with `getAssociatedTokenAddressInterface(mint, owner)` for light-token, or pass the SPL/T22 ATA to decompress to that program. `loadAta` loads all cold balance into the given ATA (no partial amount); it supports >8 inputs via batched transactions and creates the ATA if needed.
    - **Instruction-level:** Use `createLoadAtaInstructions(rpc, ata, owner, mint, payer?, interfaceOptions?, wrap?)` to get `TransactionInstruction[][]` and send batches yourself. The single-instruction primitive is no longer exported; use the batched API only.

- **CToken → LightToken renames.** Instruction and type names updated for consistency: `createAssociatedCTokenAccountInstruction` → `createAssociatedLightTokenAccountInstruction`, `CTokenConfig` → `LightTokenConfig`, `parseCTokenHot` → `parseLightTokenHot`, `parseCTokenCold` → `parseLightTokenCold`, `mintToCToken` → `mintToLightToken` (and related), `CTOKEN_PROGRAM_ID` → `LIGHT_TOKEN_PROGRAM_ID`. Use the new LightToken names in all call sites.

- **Removed exports.** `createLoadAccountsParams`, `calculateCompressibleLoadComputeUnits`, and associated types are no longer exported. Use the batched `createLoadAtaInstructions` API and `calculateLoadBatchComputeUnits` where applicable.

- **New freeze/thaw instructions.** `createLightTokenFreezeAccountInstruction` and `createLightTokenThawAccountInstruction` are available for native freeze/thaw of decompressed light-token accounts (discriminators 10 and 11).

- **Synthetic delegate selection semantics updated.** In `getAtaInterface` / `getAccountInterface` synthetic account views:
    - If a hot source has a delegate, that hot delegate is always canonical. Cold delegates only contribute to `delegatedAmount` when they match the hot delegate.
    - If there is no hot delegate, canonical delegate is chosen from the most recent delegated cold source (source-order first), and `delegatedAmount` is the sum of all cold inputs for that delegate.

## [0.23.0-beta.9]

### Fixed

- `maxTopUp` default changed from `0` (no top-ups allowed) to `MAX_TOP_UP` (65535, no cap) across all instruction builders (`wrap`, `unwrap`, `mintTo`, `createMint`, `decompressMint`, `updateMetadata`, `updateMintAuthority`). Previously rent top-ups were silently blocked, causing transaction failures on underfunded compressible accounts.
- `getSplOrToken2022AccountInterface` now fetches hot and cold accounts in parallel; individual fetch failures are handled gracefully instead of throwing immediately.
- `delegatedAmount` correctly parsed from CompressedOnly TLV extension instead of defaulting to 0.
- `parseCTokenHot` uses `unpackAccountSPL` for correct hot c-token account parsing.
- Frozen `ctoken-hot` sources now correctly excluded from load paths (was only filtering SPL/T22 frozen sources).
- SPL interface fetch errors in load paths are now rethrown when there is an SPL or T22 balance; previously all errors were silently swallowed.

### Added

- `MAX_TOP_UP` constant (65535) — exported from constants.
- `maxTopUp` optional parameter on `createWrapInstruction`, `createUnwrapInstruction`, `createMintToInstruction`, `createMintInstruction`, `decompressMintInstruction`, `createUpdateMetadataFieldInstruction`, `createUpdateMetadataAuthorityInstruction`, `createRemoveMetadataKeyInstruction`, `createUpdateMintAuthorityInstruction`, `createUpdateFreezeAuthorityInstruction` for explicit rent top-up capping.
- `maxTopUp` optional parameter on `mintToCompressed`, `unwrap`, `decompressMint` actions. **Note:** `mintToCompressed` inserts `maxTopUp` before the existing `confirmOptions` positional parameter — callers who were passing `confirmOptions` positionally must update their call sites (TypeScript will report a type error).
- `createUnwrapInstructions` — instruction builder for unwrapping, returns `TransactionInstruction[][]` with amount-aware input selection.
- `selectInputsForAmount` — greedy amount-aware compressed account selection for load/unwrap.
- `assertV2Only` guards on `loadAta` and decompress paths — V1 inputs are rejected early with a clear error.

## [0.23.0-beta.7] - Transfer Interface Hardening

### Breaking Changes

#### Renames

- **`CTOKEN_PROGRAM_ID`**: Deprecated. Use `LIGHT_TOKEN_PROGRAM_ID` (re-exported from `@lightprotocol/stateless.js`).

- **`createCTokenTransferInstruction`**: Renamed to `createLightTokenTransferInstruction`. Instruction data layout changed (see below).

- **`createTransferInterfaceInstruction`** (multi-program dispatcher): Deprecated. Use `createLightTokenTransferInstruction` for Light token transfers, or SPL's `createTransferCheckedInstruction` for SPL/T22 transfers.

#### `transferInterface` (high-level action)

- **`destination` parameter changed from ATA address to wallet public key.** The function now derives the recipient ATA internally and creates it idempotently (no extra RPC fetch). Callers that previously passed a pre-derived ATA address must now pass the recipient's wallet public key instead.

- **`programId` default changed** from `CTOKEN_PROGRAM_ID` to `LIGHT_TOKEN_PROGRAM_ID`. Parameter order unchanged: `amount, programId?, confirmOptions?, options?, wrap?`.

- **Multi-transaction support**: For >8 compressed inputs, the action now sends parallel load transactions before the final transfer transaction. Previously, all instructions were packed into a single transaction (which could exceed limits).

#### `createTransferInterfaceInstructions` (instruction builder -- NEW)

New function replacing the old monolithic `transferInterface` internals. Takes `recipient` as a wallet public key (not ATA). Returns `TransactionInstruction[][]` where each inner array is one transaction. The last element is always the transfer transaction; all preceding elements are load transactions that can be sent in parallel.

```typescript
const batches = await createTransferInterfaceInstructions(
    rpc, payer, mint, amount, sender, recipientWallet, options?,
);
const { rest: loads, last: transferTx } = sliceLast(batches);
```

Options at this point included `ensureRecipientAta` (default: `true`) and
`programId`. `ensureRecipientAta` was removed again in `0.23.0-beta.10` when
the split was introduced:
`transferInterface/createTransferInterfaceInstructions` (wallet-recipient) and
`transferToAccountInterface/createTransferToAccountInterfaceInstructions`
(explicit destination-account).

#### `createLoadAtaInstructions`

- **Return type changed** from `TransactionInstruction[]` (flat) to `TransactionInstruction[][]` (batched). Each inner array is one transaction. For >8 compressed inputs, multiple transactions are needed because each decompress proof can handle at most 8 inputs.

    ```typescript
    // Old
    const ixs: TransactionInstruction[] = await createLoadAtaInstructions(
        rpc,
        ata,
        owner,
        mint,
    );

    // New
    const batches: TransactionInstruction[][] = await createLoadAtaInstructions(
        rpc,
        ata,
        owner,
        mint,
    );
    // Each element is one transaction's instructions
    ```

#### `createLightTokenTransferInstruction` (instruction-level)

- **Instruction data layout changed**: Old format was 10 bytes (discriminator + padding + u64 LE at offset 2). New format is 9 bytes (discriminator + u64 LE at offset 1, no padding).

- **Account keys changed**: Now always includes `system_program` (index 3) and `fee_payer` (index 4) for compressible extension rent top-ups. Old format had 3 required accounts (source, destination, owner) with optional payer. New format has 5 required accounts.

- **`owner` is now writable** (for rent top-ups via compressible extension).

#### `createDecompressInterfaceInstruction`

- **New required parameter**: `decimals: number` added after `splInterfaceInfo`. Required for SPL destination decompression.

- **Delegate handling**: Now includes delegate pubkeys from input compressed accounts in the packed accounts list.

#### Program instruction: createTokenPool → createSplInterface

- **`CompressedTokenProgram.createTokenPool`**: Deprecated. Use `CompressedTokenProgram.createSplInterface` with the same call signature (`feePayer`, `mint`, `tokenProgramId?`). The high-level action `createSplInterface()` now calls the new instruction helper; the deprecated action alias `createTokenPool` still works but points to `createSplInterface`. `CompressedTokenProgram.createMint` now uses `createSplInterface` internally for the third instruction.

### Added

- **`createTransferInterfaceInstructions`**: Instruction builder for transfers with multi-transaction batching, frozen account pre-checks, zero-amount rejection, and `programId`-based dispatch (Light token vs SPL `transferChecked`).
- **`sliceLast`** helper: Splits instruction batches into `{ rest, last }` for parallel-then-sequential sending.
- **`TransferOptions`** at this point included:
  `wrap`, `programId`, `ensureRecipientAta`, extends `InterfaceOptions`.
- **Version-aware proof chunking**: V1 inputs chunked with sizes {8,4,2,1}, V2 with {8,7,6,5,4,3,2,1}. V1 and V2 never mixed in a single proof request.
- **`assertUniqueInputHashes`**: Runtime enforcement that no compressed account hash appears in more than one parallel batch.
- **`chunkAccountsByTreeVersion`**: Exported utility for splitting compressed accounts by tree version into prover-compatible groups.
- **Frozen account handling**: `_buildLoadBatches` skips frozen sources. `createTransferInterfaceInstructions` throws early if hot account is frozen, reports frozen balance in insufficient-balance errors.
- **`loadAta` action**: Now sends all load batches in parallel (previously sequential single-tx).
- **`createUnwrapInstructions`**: New instruction builder for unwrapping c-tokens to SPL/T22. Returns `TransactionInstruction[][]` (load batches, if any, then one unwrap batch). Same loop pattern as `createLoadAtaInstructions` and `createTransferInterfaceInstructions`. The `unwrap` action now uses it internally. Use this when you need instruction-level control or to handle multi-batch load + unwrap in one go.
- **`LightTokenProgram`**: Export alias for `CompressedTokenProgram` for clearer naming in docs and examples.
- **Decompress mint as part of create mint**: `createMintInterface` and the create-mint instruction now decompress the mint in the same transaction. The mint is available on-chain (CMint account created) immediately after creation; a separate `decompressMint()` call is no longer required before creating ATAs or minting. `decompressMint()` remains supported and is idempotent: if the mint was already decompressed (e.g. via `createMintInterface`), it returns successfully without sending a transaction.

## [0.22.0]

- `CreateMint` action now allows passing a non-payer mint and freeze authority.
- More efficient computebudgets for actions.
- Better DX: Parameter lookup in call signatures of CompressedTokenProgram instructions
- QoL: improved typedocs.

## [0.21.0]

#### Breaking Changes

This release has several breaking changes which improve protocol
scalability. Please reach out to the [team](https://t.me/swen_light) if you need help migrating.

### Migration guide: Compress

**Old Code** (remove this)

```typescript
const activeStateTrees = await connection.getCachedActiveStateTreeInfo();
const { tree } = pickRandomTreeAndQueue(activeStateTrees);
// ...
```

**New Code**

```typescript
// ...
const treeInfos = await rpc.getStateTreeInfos();
const treeInfo = selectStateTreeInfo(treeInfos);

const infos = await getTokenPoolInfos(rpc, mint);
const info = selectTokenPoolInfo(infos);

const compressIx = await CompressedTokenProgram.compress({
    // ...
    outputStateTreeInfo,
    tokenPoolInfo,
});
```

### Migration guide: Decompress

```typescript
// ...
// new code:
const infos = await getTokenPoolInfos(rpc, mint);
const selectedInfos = selectTokenPoolInfosForDecompression(
    tokenPoolInfos,
    amount,
);

const ix = await CompressedTokenProgram.decompress({
    // ...
    tokenPoolInfos: selectedTokenPoolInfos,
});
```

### Overview

- new type: TokenPoolInfo
- Instruction Changes:
    - `compress`, `mintTo`, `approveAndMintTo`, `compressSplTokenAccount` now require valid TokenPoolInfo
    - `decompress` now requires an array of one or more TokenPoolInfos.
    - `decompress`, `transfer` now do not allow state tree overrides.

- Action Changes:
    - Removed optional tokenProgramId: PublicKey
    - removed optional merkleTree: PublicKey
    - removed optional outputStateTree: PublicKey
    - added optional stateTreeInfo: StateTreeInfo
    - added optional tokenPoolInfo: TokenPoolInfo

- new instructions:
    - `approve`, `revoke`: delegated transfer support.
    - `addTokenPools`: you can now register additional token pool pdas. Use
      this if you need very high concurrency.

### Why the Changes are helpful

`getStateTreeInfos()` retrieves relevant info about all active state trees.

When building a transaction you can now pick a random treeInfo via `selectStateTreeInfo(infos)`.

This lets you and other apps execute more transactions within Solana's write lock
limits.

The same applies to `getTokenPoolInfos`. When you compress or decompress SPL
tokens, a tokenpool gets write-locked. If you need additional per-block write
lock capacity, you can register and sample additional (up to 4) tokenpool
accounts.

## [0.20.5-0.20.9] - 2025-02-24

### Changed

- improved documentation and error messages.

## [0.20.4] - 2025-02-19

### Breaking Changes

- `selectMinCompressedTokenAccountsForTransfer` and
  `selectSmartCompressedTokenAccountsForTransfer` now throw an error
  if not enough accounts are found. In most cases this is not a breaking
  change, because a proof request would fail anyway. This just makes the error
  message more informative.

### Added

- `selectSmartCompressedTokenAccountsForTransfer` and
  `selectSmartCompressedTokenAccountsForTransferOrPartial`

### Changed

- `selectMinCompressedTokenAccountsForTransfer` and
  `selectMinCompressedTokenAccountsForTransferorPartial` now accept an optional
  `maxInputs` parameter, defaulting to 4.

### Security

- N/A

For previous release notes, check:
https://www.zkcompression.com/release-notes/1.0.0-mainnet-beta
