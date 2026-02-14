/**
 * Light Protocol Token Client
 *
 * Indexer client and account loading functions for compressed tokens.
 *
 * @example
 * ```typescript
 * import {
 *   createLightIndexer,
 *   loadTokenAccountsForTransfer,
 *   selectAccountsForAmount,
 * } from '@lightprotocol/token-client';
 *
 * // Types from token-sdk:
 * import { TreeType, CompressedTokenAccount } from '@lightprotocol/token-sdk';
 *
 * const indexer = createLightIndexer('https://photon.helius.dev');
 * const loaded = await loadTokenAccountsForTransfer(indexer, owner, 1000n);
 * ```
 *
 * @packageDocumentation
 */

// Indexer
export {
    type LightIndexer,
    PhotonIndexer,
    createLightIndexer,
    isLightIndexerAvailable,
} from './indexer.js';

// Load functions
export {
    // Types
    type InputTokenAccount,
    type MerkleContext,
    type LoadedTokenAccounts,
    type LoadTokenAccountsOptions,
    type SelectedAccounts,

    // Load functions
    loadTokenAccountsForTransfer,
    loadTokenAccount,
    loadAllTokenAccounts,
    loadCompressedAccount,
    loadCompressedAccountByHash,

    // Account selection
    selectAccountsForAmount,
    DEFAULT_MAX_INPUTS,

    // Proof helpers
    getValidityProofForAccounts,
    needsValidityProof,
    getTreeInfo,
    getOutputTreeInfo,
} from './load.js';

// Actions (high-level builders)
export {
    buildCompressedTransfer,
    type BuildTransferResult,
} from './actions.js';
