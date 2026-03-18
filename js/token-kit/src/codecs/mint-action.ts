/**
 * MintAction instruction codecs using Solana Kit patterns.
 *
 * Handles encoding of MintAction instruction data (discriminator 103).
 * Uses manual Borsh encoding via DataView/Uint8Array for complex nested
 * structures, following the same approach as transfer2.ts.
 */

import type { ReadonlyUint8Array } from '@solana/codecs';

import {
    writeU8,
    writeU16,
    writeU32,
    writeU64,
    writeBool,
    writeOption,
    writeVecBytes,
    concatBytes,
} from './borsh-helpers.js';

import type {
    CompressedProof,
    ExtensionInstructionData,
} from './types.js';

import { encodeExtensionInstructionData } from './transfer2.js';
import { DISCRIMINATOR } from '../constants.js';

// ============================================================================
// MINT ACTION TYPES
// ============================================================================

/** Recipient for MintToCompressed action. */
export interface MintRecipient {
    /** Recipient pubkey (32 bytes). */
    recipient: ReadonlyUint8Array;
    /** Amount to mint. */
    amount: bigint;
}

/** Mint compressed tokens to compressed accounts. */
export interface MintToCompressedAction {
    type: 'MintToCompressed';
    /** Token account version. */
    tokenAccountVersion: number;
    /** Recipients to mint to. */
    recipients: MintRecipient[];
}

/** Mint tokens from a compressed mint to a token Solana account. */
export interface MintToAction {
    type: 'MintTo';
    /** Index into remaining accounts for the recipient token account. */
    accountIndex: number;
    /** Amount to mint. */
    amount: bigint;
}

/** Update mint authority or freeze authority of a compressed mint. */
export interface UpdateAuthorityAction {
    type: 'UpdateMintAuthority' | 'UpdateFreezeAuthority';
    /** New authority (32 bytes), or null to revoke. */
    newAuthority: ReadonlyUint8Array | null;
}

/** Update a metadata field on a compressed mint. */
export interface UpdateMetadataFieldAction {
    type: 'UpdateMetadataField';
    /** Index of the TokenMetadata extension in the extensions array. */
    extensionIndex: number;
    /** Field type: 0=Name, 1=Symbol, 2=Uri, 3=Custom key. */
    fieldType: number;
    /** Empty for Name/Symbol/Uri, key string bytes for custom fields. */
    key: ReadonlyUint8Array;
    /** UTF-8 encoded value. */
    value: ReadonlyUint8Array;
}

/** Update metadata authority on a compressed mint. */
export interface UpdateMetadataAuthorityAction {
    type: 'UpdateMetadataAuthority';
    /** Index of the TokenMetadata extension in the extensions array. */
    extensionIndex: number;
    /** New authority (use zero bytes to set to None). */
    newAuthority: ReadonlyUint8Array;
}

/** Remove a metadata key from a compressed mint. */
export interface RemoveMetadataKeyAction {
    type: 'RemoveMetadataKey';
    /** Index of the TokenMetadata extension in the extensions array. */
    extensionIndex: number;
    /** UTF-8 encoded key to remove. */
    key: ReadonlyUint8Array;
    /** 0=false, 1=true - don't error if key doesn't exist. */
    idempotent: number;
}

/** Decompress a compressed mint to a Mint Solana account. */
export interface DecompressMintAction {
    type: 'DecompressMint';
    /** Rent payment in epochs (prepaid, must be >= 2). */
    rentPayment: number;
    /** Lamports allocated for future write operations. */
    writeTopUp: number;
}

/** Compress and close a Mint Solana account. */
export interface CompressAndCloseMintAction {
    type: 'CompressAndCloseMint';
    /** If non-zero, succeed silently when Mint doesn't exist or cannot be compressed. */
    idempotent: number;
}

/** Union of all MintAction variants. */
export type MintAction =
    | MintToCompressedAction
    | UpdateAuthorityAction
    | MintToAction
    | UpdateMetadataFieldAction
    | UpdateMetadataAuthorityAction
    | RemoveMetadataKeyAction
    | DecompressMintAction
    | CompressAndCloseMintAction;

// ============================================================================
// CREATE MINT & MINT METADATA
// ============================================================================

/** CreateMint data for new mint creation. */
export interface CreateMint {
    /** Placeholder for address trees (4 bytes, currently all zeros). */
    readOnlyAddressTrees: Uint8Array;
    /** Placeholder for root indices (4 x u16, currently all zeros). */
    readOnlyAddressTreeRootIndices: number[];
}

/** Light Protocol-specific metadata for compressed mints. */
export interface MintMetadata {
    /** Version for upgradability. */
    version: number;
    /** Whether the mint has been decompressed. */
    mintDecompressed: boolean;
    /** PDA derived from mintSigner (32 bytes). */
    mint: ReadonlyUint8Array;
    /** Signer pubkey used to derive the mint PDA (32 bytes). */
    mintSigner: ReadonlyUint8Array;
    /** Bump seed from mint PDA derivation. */
    bump: number;
}

/** Mint instruction data for creating or updating a mint. */
export interface MintInstructionData {
    /** Total supply of tokens. */
    supply: bigint;
    /** Number of base 10 digits to the right of the decimal place. */
    decimals: number;
    /** Light Protocol-specific metadata. */
    metadata: MintMetadata;
    /** Optional mint authority (32 bytes). */
    mintAuthority: ReadonlyUint8Array | null;
    /** Optional freeze authority (32 bytes). */
    freezeAuthority: ReadonlyUint8Array | null;
    /** Optional extensions for additional functionality. */
    extensions: ExtensionInstructionData[] | null;
}

// ============================================================================
// CPI CONTEXT
// ============================================================================

/** CPI context for mint action operations. */
export interface MintActionCpiContext {
    /** Whether to set the CPI context. */
    setContext: boolean;
    /** Whether this is the first set context call. */
    firstSetContext: boolean;
    /** Address tree index if create mint. */
    inTreeIndex: number;
    /** Input queue index. */
    inQueueIndex: number;
    /** Output queue index. */
    outQueueIndex: number;
    /** Token output queue index. */
    tokenOutQueueIndex: number;
    /** Index of the compressed account that should receive the new address. */
    assignedAccountIndex: number;
    /** Placeholder for multiple address trees (4 bytes). */
    readOnlyAddressTrees: Uint8Array;
    /** Address tree pubkey (32 bytes). */
    addressTreePubkey: ReadonlyUint8Array;
}

// ============================================================================
// TOP-LEVEL INSTRUCTION DATA
// ============================================================================

/** Full MintAction instruction data (discriminator 103). */
export interface MintActionInstructionData {
    /** Leaf index in the merkle tree (only set if mint already exists). */
    leafIndex: number;
    /** Whether to prove by index (only set if mint already exists). */
    proveByIndex: boolean;
    /** Root index for address or validity proof. */
    rootIndex: number;
    /** Maximum lamports for rent and top-up (u16::MAX = no limit, 0 = no top-ups). */
    maxTopUp: number;
    /** Only set when creating a new mint. */
    createMint: CreateMint | null;
    /** Actions to perform on the mint. */
    actions: MintAction[];
    /** Validity proof (optional). */
    proof: CompressedProof | null;
    /** CPI context (optional). */
    cpiContext: MintActionCpiContext | null;
    /** Mint data (optional, for create or full state). */
    mint: MintInstructionData | null;
}

// ============================================================================
// ACTION ENCODERS
// ============================================================================

/**
 * Borsh enum variant indices for the Action enum, matching the Rust definition.
 */
const ACTION_VARIANT = {
    MintToCompressed: 0,
    UpdateMintAuthority: 1,
    UpdateFreezeAuthority: 2,
    MintTo: 3,
    UpdateMetadataField: 4,
    UpdateMetadataAuthority: 5,
    RemoveMetadataKey: 6,
    DecompressMint: 7,
    CompressAndCloseMint: 8,
} as const;

function encodeMintRecipient(r: MintRecipient): Uint8Array {
    return concatBytes([
        new Uint8Array(r.recipient),
        writeU64(r.amount),
    ]);
}

function encodeMintToCompressedAction(
    action: MintToCompressedAction,
): Uint8Array {
    const parts: Uint8Array[] = [
        writeU8(action.tokenAccountVersion),
        writeU32(action.recipients.length),
    ];
    for (const r of action.recipients) {
        parts.push(encodeMintRecipient(r));
    }
    return concatBytes(parts);
}

function encodeUpdateAuthority(newAuthority: ReadonlyUint8Array | null): Uint8Array {
    return writeOption(newAuthority, (v: ReadonlyUint8Array) =>
        new Uint8Array(v),
    );
}

function encodeMintToAction(action: MintToAction): Uint8Array {
    return concatBytes([
        writeU8(action.accountIndex),
        writeU64(action.amount),
    ]);
}

function encodeUpdateMetadataFieldAction(
    action: UpdateMetadataFieldAction,
): Uint8Array {
    return concatBytes([
        writeU8(action.extensionIndex),
        writeU8(action.fieldType),
        writeVecBytes(action.key),
        writeVecBytes(action.value),
    ]);
}

function encodeUpdateMetadataAuthorityAction(
    action: UpdateMetadataAuthorityAction,
): Uint8Array {
    return concatBytes([
        writeU8(action.extensionIndex),
        new Uint8Array(action.newAuthority),
    ]);
}

function encodeRemoveMetadataKeyAction(
    action: RemoveMetadataKeyAction,
): Uint8Array {
    return concatBytes([
        writeU8(action.extensionIndex),
        writeVecBytes(action.key),
        writeU8(action.idempotent),
    ]);
}

function encodeDecompressMintAction(
    action: DecompressMintAction,
): Uint8Array {
    return concatBytes([
        writeU8(action.rentPayment),
        writeU32(action.writeTopUp),
    ]);
}

function encodeCompressAndCloseMintAction(
    action: CompressAndCloseMintAction,
): Uint8Array {
    return writeU8(action.idempotent);
}

function encodeAction(action: MintAction): Uint8Array {
    switch (action.type) {
        case 'MintToCompressed':
            return concatBytes([
                writeU8(ACTION_VARIANT.MintToCompressed),
                encodeMintToCompressedAction(action),
            ]);
        case 'UpdateMintAuthority':
            return concatBytes([
                writeU8(ACTION_VARIANT.UpdateMintAuthority),
                encodeUpdateAuthority(action.newAuthority),
            ]);
        case 'UpdateFreezeAuthority':
            return concatBytes([
                writeU8(ACTION_VARIANT.UpdateFreezeAuthority),
                encodeUpdateAuthority(action.newAuthority),
            ]);
        case 'MintTo':
            return concatBytes([
                writeU8(ACTION_VARIANT.MintTo),
                encodeMintToAction(action),
            ]);
        case 'UpdateMetadataField':
            return concatBytes([
                writeU8(ACTION_VARIANT.UpdateMetadataField),
                encodeUpdateMetadataFieldAction(action),
            ]);
        case 'UpdateMetadataAuthority':
            return concatBytes([
                writeU8(ACTION_VARIANT.UpdateMetadataAuthority),
                encodeUpdateMetadataAuthorityAction(action),
            ]);
        case 'RemoveMetadataKey':
            return concatBytes([
                writeU8(ACTION_VARIANT.RemoveMetadataKey),
                encodeRemoveMetadataKeyAction(action),
            ]);
        case 'DecompressMint':
            return concatBytes([
                writeU8(ACTION_VARIANT.DecompressMint),
                encodeDecompressMintAction(action),
            ]);
        case 'CompressAndCloseMint':
            return concatBytes([
                writeU8(ACTION_VARIANT.CompressAndCloseMint),
                encodeCompressAndCloseMintAction(action),
            ]);
    }
}

// ============================================================================
// STRUCT ENCODERS
// ============================================================================

function encodeCreateMint(data: CreateMint): Uint8Array {
    const parts: Uint8Array[] = [
        new Uint8Array(data.readOnlyAddressTrees),
    ];
    for (const idx of data.readOnlyAddressTreeRootIndices) {
        parts.push(writeU16(idx));
    }
    return concatBytes(parts);
}

function encodeMintMetadata(data: MintMetadata): Uint8Array {
    return concatBytes([
        writeU8(data.version),
        writeBool(data.mintDecompressed),
        new Uint8Array(data.mint),
        new Uint8Array(data.mintSigner),
        writeU8(data.bump),
    ]);
}

function encodeMintInstructionData(data: MintInstructionData): Uint8Array {
    const parts: Uint8Array[] = [
        writeU64(data.supply),
        writeU8(data.decimals),
        encodeMintMetadata(data.metadata),
        writeOption(data.mintAuthority, (v: ReadonlyUint8Array) =>
            new Uint8Array(v),
        ),
        writeOption(data.freezeAuthority, (v: ReadonlyUint8Array) =>
            new Uint8Array(v),
        ),
    ];

    // Option<Vec<ExtensionInstructionData>>
    if (data.extensions === null) {
        parts.push(new Uint8Array([0]));
    } else {
        parts.push(new Uint8Array([1]));
        parts.push(writeU32(data.extensions.length));
        for (const ext of data.extensions) {
            parts.push(encodeExtensionInstructionData(ext));
        }
    }

    return concatBytes(parts);
}

function encodeCompressedProof(proof: CompressedProof): Uint8Array {
    return concatBytes([
        new Uint8Array(proof.a),
        new Uint8Array(proof.b),
        new Uint8Array(proof.c),
    ]);
}

function encodeMintActionCpiContext(
    ctx: MintActionCpiContext,
): Uint8Array {
    return concatBytes([
        writeBool(ctx.setContext),
        writeBool(ctx.firstSetContext),
        writeU8(ctx.inTreeIndex),
        writeU8(ctx.inQueueIndex),
        writeU8(ctx.outQueueIndex),
        writeU8(ctx.tokenOutQueueIndex),
        writeU8(ctx.assignedAccountIndex),
        new Uint8Array(ctx.readOnlyAddressTrees),
        new Uint8Array(ctx.addressTreePubkey),
    ]);
}

// ============================================================================
// TOP-LEVEL ENCODER
// ============================================================================

/**
 * Encodes the full MintAction instruction data including discriminator (103).
 *
 * Borsh layout:
 * - discriminator: u8 (103)
 * - leaf_index: u32
 * - prove_by_index: bool
 * - root_index: u16
 * - max_top_up: u16
 * - create_mint: Option<CreateMint>
 * - actions: Vec<Action>
 * - proof: Option<CompressedProof>
 * - cpi_context: Option<CpiContext>
 * - mint: Option<MintInstructionData>
 */
export function encodeMintActionInstructionData(
    data: MintActionInstructionData,
): Uint8Array {
    const parts: Uint8Array[] = [
        // Discriminator
        writeU8(DISCRIMINATOR.MINT_ACTION),

        // Base fields
        writeU32(data.leafIndex),
        writeBool(data.proveByIndex),
        writeU16(data.rootIndex),
        writeU16(data.maxTopUp),

        // Option<CreateMint>
        writeOption(data.createMint, encodeCreateMint),

        // Vec<Action>
        writeU32(data.actions.length),
    ];

    for (const action of data.actions) {
        parts.push(encodeAction(action));
    }

    // Option<CompressedProof>
    parts.push(writeOption(data.proof, encodeCompressedProof));
    parts.push(writeOption(data.cpiContext, encodeMintActionCpiContext));
    parts.push(writeOption(data.mint, encodeMintInstructionData));

    return concatBytes(parts);
}
