/**
 * Light Token SDK Client Types
 *
 * Types for interacting with the Light Protocol indexer (Photon).
 * Implementation moved to @lightprotocol/token-client package.
 */

// Types only - implementation in @lightprotocol/token-client
export {
    // Tree types
    TreeType,
    type TreeInfo,

    // Account types
    AccountState,
    type CompressedAccountData,
    type CompressedAccount,
    type TokenData,
    type CompressedTokenAccount,

    // Proof types
    type ValidityProof,
    type RootIndex,
    type AccountProofInputs,
    type AddressProofInputs,
    type ValidityProofWithContext,

    // Request/response types
    type AddressWithTree,
    type GetCompressedTokenAccountsOptions,
    type ResponseContext,
    type IndexerResponse,
    type ItemsWithCursor,

    // Error types
    IndexerErrorCode,
    IndexerError,

    // Validation
    assertV2Tree,
} from './types.js';
