/**
 * Light Token SDK Client Types
 *
 * Core types for interacting with the Light Protocol indexer (Photon).
 * These types align with the Rust sdk-libs/client types.
 */

import type { Address } from '@solana/addresses';

// ============================================================================
// TREE TYPES
// ============================================================================

/**
 * Tree type enum matching Rust TreeType.
 */
export enum TreeType {
    /** V1 state merkle tree */
    StateV1 = 1,
    /** V1 address merkle tree */
    AddressV1 = 2,
    /** V2 state merkle tree */
    StateV2 = 3,
    /** V2 address merkle tree */
    AddressV2 = 4,
}

/**
 * Tree info for a merkle tree context.
 */
export interface TreeInfo {
    /** Merkle tree pubkey */
    tree: Address;
    /** Queue pubkey */
    queue: Address;
    /** Tree type */
    treeType: TreeType;
    /** CPI context (optional) */
    cpiContext?: Address;
    /** Next tree info (when current tree is full) */
    nextTreeInfo?: TreeInfo | null;
}

// ============================================================================
// ACCOUNT TYPES
// ============================================================================

/**
 * Account state for token accounts.
 */
export enum AccountState {
    Initialized = 0,
    Frozen = 1,
}

/**
 * Compressed account data.
 */
export interface CompressedAccountData {
    /** 8-byte discriminator */
    discriminator: Uint8Array;
    /** Account data bytes */
    data: Uint8Array;
    /** 32-byte data hash */
    dataHash: Uint8Array;
}

/**
 * Compressed account matching Rust CompressedAccount.
 */
export interface CompressedAccount {
    /** 32-byte account hash */
    hash: Uint8Array;
    /** 32-byte address (optional) */
    address: Uint8Array | null;
    /** Owner program pubkey */
    owner: Address;
    /** Lamports */
    lamports: bigint;
    /** Account data (optional) */
    data: CompressedAccountData | null;
    /** Leaf index in the merkle tree */
    leafIndex: number;
    /** Tree info */
    treeInfo: TreeInfo;
    /** Whether to prove by index */
    proveByIndex: boolean;
    /** Sequence number (optional) */
    seq: bigint | null;
    /** Slot when account was created */
    slotCreated: bigint;
}

/**
 * Token-specific data.
 */
export interface TokenData {
    /** Token mint */
    mint: Address;
    /** Token owner */
    owner: Address;
    /** Token amount */
    amount: bigint;
    /** Delegate (optional) */
    delegate: Address | null;
    /** Account state */
    state: AccountState;
    /** TLV extension data (optional) */
    tlv: Uint8Array | null;
}

/**
 * Compressed token account combining account and token data.
 */
export interface CompressedTokenAccount {
    /** Token-specific data */
    token: TokenData;
    /** General account information */
    account: CompressedAccount;
}

// ============================================================================
// PROOF TYPES
// ============================================================================

/**
 * Groth16 validity proof.
 */
export interface ValidityProof {
    /** 32 bytes - G1 point */
    a: Uint8Array;
    /** 64 bytes - G2 point */
    b: Uint8Array;
    /** 32 bytes - G1 point */
    c: Uint8Array;
}

/**
 * Root index for proof context.
 */
export interface RootIndex {
    /** The root index value */
    rootIndex: number;
    /** Whether to prove by index rather than validity proof */
    proveByIndex: boolean;
}

/**
 * Account proof inputs for validity proof context.
 */
export interface AccountProofInputs {
    /** 32-byte account hash */
    hash: Uint8Array;
    /** 32-byte merkle root */
    root: Uint8Array;
    /** Root index info */
    rootIndex: RootIndex;
    /** Leaf index */
    leafIndex: number;
    /** Tree info */
    treeInfo: TreeInfo;
}

/**
 * Address proof inputs for validity proof context.
 */
export interface AddressProofInputs {
    /** 32-byte address */
    address: Uint8Array;
    /** 32-byte merkle root */
    root: Uint8Array;
    /** Root index */
    rootIndex: number;
    /** Tree info */
    treeInfo: TreeInfo;
}

/**
 * Validity proof with full context.
 */
export interface ValidityProofWithContext {
    /** The validity proof (null if proving by index) */
    proof: ValidityProof | null;
    /** Account proof inputs */
    accounts: AccountProofInputs[];
    /** Address proof inputs */
    addresses: AddressProofInputs[];
}

// ============================================================================
// REQUEST/RESPONSE TYPES
// ============================================================================

/**
 * Address with tree for new address proofs.
 */
export interface AddressWithTree {
    /** 32-byte address */
    address: Uint8Array;
    /** Address tree pubkey */
    tree: Address;
}

/**
 * Options for fetching compressed token accounts.
 */
export interface GetCompressedTokenAccountsOptions {
    /** Filter by mint */
    mint?: Address;
    /** Pagination cursor */
    cursor?: string;
    /** Maximum results to return */
    limit?: number;
}

/**
 * Response context with slot.
 */
export interface ResponseContext {
    /** Slot of the response */
    slot: bigint;
}

/**
 * Response wrapper with context.
 */
export interface IndexerResponse<T> {
    /** Response context */
    context: ResponseContext;
    /** Response value */
    value: T;
}

/**
 * Paginated items with cursor.
 */
export interface ItemsWithCursor<T> {
    /** Items in this page */
    items: T[];
    /** Cursor for next page (null if no more pages) */
    cursor: string | null;
}

// ============================================================================
// ERROR TYPES
// ============================================================================

/**
 * Indexer error codes.
 */
export enum IndexerErrorCode {
    /** Network/fetch error */
    NetworkError = 'NETWORK_ERROR',
    /** Invalid response format */
    InvalidResponse = 'INVALID_RESPONSE',
    /** RPC error response */
    RpcError = 'RPC_ERROR',
    /** Account not found */
    NotFound = 'NOT_FOUND',
    /** Insufficient balance for operation */
    InsufficientBalance = 'INSUFFICIENT_BALANCE',
}

/**
 * Error from indexer operations.
 */
export class IndexerError extends Error {
    constructor(
        public readonly code: IndexerErrorCode,
        message: string,
        public readonly cause?: unknown,
    ) {
        super(message);
        this.name = 'IndexerError';
    }
}

// ============================================================================
// VALIDATION
// ============================================================================

/**
 * Assert that tree is V2. Throws if V1.
 *
 * The SDK only supports V2 trees. V1 trees from the indexer response
 * must be rejected to ensure proper protocol compatibility.
 *
 * @param treeType - The tree type to validate
 * @throws IndexerError if tree type is V1
 */
export function assertV2Tree(treeType: TreeType): void {
    if (treeType === TreeType.StateV1 || treeType === TreeType.AddressV1) {
        throw new IndexerError(
            IndexerErrorCode.InvalidResponse,
            `V1 tree types are not supported. Got: ${TreeType[treeType]}`,
        );
    }
}
