/**
 * Borsh layouts for MintAction instruction data
 *
 * These layouts match the Rust structs in:
 * program-libs/ctoken-types/src/instructions/mint_action/
 *
 * @module mint-action-layout
 */
import { PublicKey } from '@solana/web3.js';
import { Buffer } from 'buffer';
import BN from 'bn.js';
import {
    struct,
    option,
    vec,
    bool,
    u8,
    u16,
    u32,
    u64,
    array,
    vecU8,
    publicKey,
    rustEnum,
} from '@coral-xyz/borsh';

// ============================================================================
// Constants
// ============================================================================

export const MINT_ACTION_DISCRIMINATOR = Buffer.from([103]);

// ============================================================================
// Sub-layouts for Action variants
// ============================================================================

/** Recipient { recipient: Pubkey, amount: u64 } */
export const RecipientLayout = struct([
    publicKey('recipient'),
    u64('amount'),
]);

/** MintToCompressedAction { token_account_version: u8, recipients: Vec<Recipient> } */
export const MintToCompressedActionLayout = struct([
    u8('tokenAccountVersion'),
    vec(RecipientLayout, 'recipients'),
]);

/** UpdateAuthority { new_authority: Option<Pubkey> } */
export const UpdateAuthorityLayout = struct([
    option(publicKey(), 'newAuthority'),
]);

/** CreateSplMintAction { mint_bump: u8 } */
export const CreateSplMintActionLayout = struct([u8('mintBump')]);

/** MintToCTokenAction { account_index: u8, amount: u64 } */
export const MintToCTokenActionLayout = struct([
    u8('accountIndex'),
    u64('amount'),
]);

/** UpdateMetadataFieldAction { extension_index: u8, field_type: u8, key: Vec<u8>, value: Vec<u8> } */
export const UpdateMetadataFieldActionLayout = struct([
    u8('extensionIndex'),
    u8('fieldType'),
    vecU8('key'),
    vecU8('value'),
]);

/** UpdateMetadataAuthorityAction { extension_index: u8, new_authority: Pubkey } */
export const UpdateMetadataAuthorityActionLayout = struct([
    u8('extensionIndex'),
    publicKey('newAuthority'),
]);

/** RemoveMetadataKeyAction { extension_index: u8, key: Vec<u8>, idempotent: u8 } */
export const RemoveMetadataKeyActionLayout = struct([
    u8('extensionIndex'),
    vecU8('key'),
    u8('idempotent'),
]);

// ============================================================================
// Action enum layout
// ============================================================================

/**
 * Action enum (Rust):
 * 0 = MintToCompressed(MintToCompressedAction)
 * 1 = UpdateMintAuthority(UpdateAuthority)
 * 2 = UpdateFreezeAuthority(UpdateAuthority)
 * 3 = CreateSplMint(CreateSplMintAction)
 * 4 = MintToCToken(MintToCTokenAction)
 * 5 = UpdateMetadataField(UpdateMetadataFieldAction)
 * 6 = UpdateMetadataAuthority(UpdateMetadataAuthorityAction)
 * 7 = RemoveMetadataKey(RemoveMetadataKeyAction)
 */
export const ActionLayout = rustEnum([
    MintToCompressedActionLayout.replicate('mintToCompressed'),
    UpdateAuthorityLayout.replicate('updateMintAuthority'),
    UpdateAuthorityLayout.replicate('updateFreezeAuthority'),
    CreateSplMintActionLayout.replicate('createSplMint'),
    MintToCTokenActionLayout.replicate('mintToCToken'),
    UpdateMetadataFieldActionLayout.replicate('updateMetadataField'),
    UpdateMetadataAuthorityActionLayout.replicate('updateMetadataAuthority'),
    RemoveMetadataKeyActionLayout.replicate('removeMetadataKey'),
]);

// ============================================================================
// CompressedProof layout
// ============================================================================

/** CompressedProof { a: [u8; 32], b: [u8; 64], c: [u8; 32] } */
export const CompressedProofLayout = struct([
    array(u8(), 32, 'a'),
    array(u8(), 64, 'b'),
    array(u8(), 32, 'c'),
]);

// ============================================================================
// CpiContext layout
// ============================================================================

/**
 * CpiContext {
 *   set_context: bool,
 *   first_set_context: bool,
 *   in_tree_index: u8,
 *   in_queue_index: u8,
 *   out_queue_index: u8,
 *   token_out_queue_index: u8,
 *   assigned_account_index: u8,
 *   read_only_address_trees: [u8; 4],
 *   address_tree_pubkey: [u8; 32],
 * }
 */
export const CpiContextLayout = struct([
    bool('setContext'),
    bool('firstSetContext'),
    u8('inTreeIndex'),
    u8('inQueueIndex'),
    u8('outQueueIndex'),
    u8('tokenOutQueueIndex'),
    u8('assignedAccountIndex'),
    array(u8(), 4, 'readOnlyAddressTrees'),
    array(u8(), 32, 'addressTreePubkey'),
]);

// ============================================================================
// CreateMint layout
// ============================================================================

/**
 * CreateMint {
 *   read_only_address_trees: [u8; 4],
 *   read_only_address_tree_root_indices: [u16; 4],
 * }
 */
export const CreateMintLayout = struct([
    array(u8(), 4, 'readOnlyAddressTrees'),
    array(u16(), 4, 'readOnlyAddressTreeRootIndices'),
]);

// ============================================================================
// AdditionalMetadata layout
// ============================================================================

/** AdditionalMetadata { key: Vec<u8>, value: Vec<u8> } */
export const AdditionalMetadataLayout = struct([
    vecU8('key'),
    vecU8('value'),
]);

// ============================================================================
// TokenMetadataInstructionData layout
// ============================================================================

/**
 * TokenMetadataInstructionData {
 *   update_authority: Option<Pubkey>,
 *   name: Vec<u8>,
 *   symbol: Vec<u8>,
 *   uri: Vec<u8>,
 *   additional_metadata: Option<Vec<AdditionalMetadata>>,
 * }
 */
export const TokenMetadataInstructionDataLayout = struct([
    option(publicKey(), 'updateAuthority'),
    vecU8('name'),
    vecU8('symbol'),
    vecU8('uri'),
    option(vec(AdditionalMetadataLayout), 'additionalMetadata'),
]);

// ============================================================================
// ExtensionInstructionData enum layout
// ============================================================================

/**
 * ExtensionInstructionData enum (Rust):
 * 0-18 = Placeholder variants
 * 19 = TokenMetadata(TokenMetadataInstructionData)
 *
 * We use rustEnum with placeholders for discriminants 0-18
 */
const PlaceholderLayout = struct([]);

export const ExtensionInstructionDataLayout = rustEnum([
    PlaceholderLayout.replicate('placeholder0'),
    PlaceholderLayout.replicate('placeholder1'),
    PlaceholderLayout.replicate('placeholder2'),
    PlaceholderLayout.replicate('placeholder3'),
    PlaceholderLayout.replicate('placeholder4'),
    PlaceholderLayout.replicate('placeholder5'),
    PlaceholderLayout.replicate('placeholder6'),
    PlaceholderLayout.replicate('placeholder7'),
    PlaceholderLayout.replicate('placeholder8'),
    PlaceholderLayout.replicate('placeholder9'),
    PlaceholderLayout.replicate('placeholder10'),
    PlaceholderLayout.replicate('placeholder11'),
    PlaceholderLayout.replicate('placeholder12'),
    PlaceholderLayout.replicate('placeholder13'),
    PlaceholderLayout.replicate('placeholder14'),
    PlaceholderLayout.replicate('placeholder15'),
    PlaceholderLayout.replicate('placeholder16'),
    PlaceholderLayout.replicate('placeholder17'),
    PlaceholderLayout.replicate('placeholder18'),
    TokenMetadataInstructionDataLayout.replicate('tokenMetadata'),
]);

// ============================================================================
// CompressedMintMetadata layout
// ============================================================================

/**
 * CompressedMintMetadata {
 *   version: u8,
 *   spl_mint_initialized: bool,
 *   mint: Pubkey,
 * }
 */
export const CompressedMintMetadataLayout = struct([
    u8('version'),
    bool('splMintInitialized'),
    publicKey('mint'),
]);

// ============================================================================
// CompressedMintInstructionData layout
// ============================================================================

/**
 * CompressedMintInstructionData {
 *   supply: u64,
 *   decimals: u8,
 *   metadata: CompressedMintMetadata,
 *   mint_authority: Option<Pubkey>,
 *   freeze_authority: Option<Pubkey>,
 *   extensions: Option<Vec<ExtensionInstructionData>>,
 * }
 */
export const CompressedMintInstructionDataLayout = struct([
    u64('supply'),
    u8('decimals'),
    CompressedMintMetadataLayout.replicate('metadata'),
    option(publicKey(), 'mintAuthority'),
    option(publicKey(), 'freezeAuthority'),
    option(vec(ExtensionInstructionDataLayout), 'extensions'),
]);

// ============================================================================
// MintActionCompressedInstructionData layout
// ============================================================================

/**
 * MintActionCompressedInstructionData {
 *   leaf_index: u32,
 *   prove_by_index: bool,
 *   root_index: u16,
 *   compressed_address: [u8; 32],
 *   token_pool_bump: u8,
 *   token_pool_index: u8,
 *   create_mint: Option<CreateMint>,
 *   actions: Vec<Action>,
 *   proof: Option<CompressedProof>,
 *   cpi_context: Option<CpiContext>,
 *   mint: CompressedMintInstructionData,
 * }
 */
export const MintActionCompressedInstructionDataLayout = struct([
    u32('leafIndex'),
    bool('proveByIndex'),
    u16('rootIndex'),
    array(u8(), 32, 'compressedAddress'),
    u8('tokenPoolBump'),
    u8('tokenPoolIndex'),
    option(CreateMintLayout, 'createMint'),
    vec(ActionLayout, 'actions'),
    option(CompressedProofLayout, 'proof'),
    option(CpiContextLayout, 'cpiContext'),
    CompressedMintInstructionDataLayout.replicate('mint'),
]);

// ============================================================================
// Types for instruction encoding
// ============================================================================

export interface ValidityProof {
    a: number[];
    b: number[];
    c: number[];
}

export interface Recipient {
    recipient: PublicKey;
    amount: bigint;
}

export interface MintToCompressedAction {
    tokenAccountVersion: number;
    recipients: Recipient[];
}

export interface UpdateAuthority {
    newAuthority: PublicKey | null;
}

export interface CreateSplMintAction {
    mintBump: number;
}

export interface MintToCTokenAction {
    accountIndex: number;
    amount: bigint;
}

export interface UpdateMetadataFieldAction {
    extensionIndex: number;
    fieldType: number;
    key: Buffer;
    value: Buffer;
}

export interface UpdateMetadataAuthorityAction {
    extensionIndex: number;
    newAuthority: PublicKey;
}

export interface RemoveMetadataKeyAction {
    extensionIndex: number;
    key: Buffer;
    idempotent: number;
}

export type Action =
    | { mintToCompressed: MintToCompressedAction }
    | { updateMintAuthority: UpdateAuthority }
    | { updateFreezeAuthority: UpdateAuthority }
    | { createSplMint: CreateSplMintAction }
    | { mintToCToken: MintToCTokenAction }
    | { updateMetadataField: UpdateMetadataFieldAction }
    | { updateMetadataAuthority: UpdateMetadataAuthorityAction }
    | { removeMetadataKey: RemoveMetadataKeyAction };

export interface CpiContext {
    setContext: boolean;
    firstSetContext: boolean;
    inTreeIndex: number;
    inQueueIndex: number;
    outQueueIndex: number;
    tokenOutQueueIndex: number;
    assignedAccountIndex: number;
    readOnlyAddressTrees: number[];
    addressTreePubkey: number[];
}

export interface CreateMint {
    readOnlyAddressTrees: number[];
    readOnlyAddressTreeRootIndices: number[];
}

export interface AdditionalMetadata {
    key: Buffer;
    value: Buffer;
}

export interface TokenMetadataInstructionData {
    updateAuthority: PublicKey | null;
    name: Buffer;
    symbol: Buffer;
    uri: Buffer;
    additionalMetadata: AdditionalMetadata[] | null;
}

export type ExtensionInstructionData = { tokenMetadata: TokenMetadataInstructionData };

export interface CompressedMintMetadata {
    version: number;
    splMintInitialized: boolean;
    mint: PublicKey;
}

export interface CompressedMintInstructionData {
    supply: bigint;
    decimals: number;
    metadata: CompressedMintMetadata;
    mintAuthority: PublicKey | null;
    freezeAuthority: PublicKey | null;
    extensions: ExtensionInstructionData[] | null;
}

export interface MintActionCompressedInstructionData {
    leafIndex: number;
    proveByIndex: boolean;
    rootIndex: number;
    compressedAddress: number[];
    tokenPoolBump: number;
    tokenPoolIndex: number;
    createMint: CreateMint | null;
    actions: Action[];
    proof: ValidityProof | null;
    cpiContext: CpiContext | null;
    mint: CompressedMintInstructionData;
}

// ============================================================================
// Encoding function
// ============================================================================

/**
 * Convert bigint to BN for Borsh encoding
 */
function toBN(value: bigint | BN | number): BN {
    if (BN.isBN(value)) return value;
    if (typeof value === 'bigint') return new BN(value.toString());
    return new BN(value);
}

/**
 * Encode MintActionCompressedInstructionData to buffer
 *
 * @param data - The instruction data to encode
 * @returns Encoded buffer with discriminator prepended
 */
export function encodeMintActionInstructionData(
    data: MintActionCompressedInstructionData,
): Buffer {
    // Convert bigint fields to BN for Borsh encoding
    const encodableData = {
        ...data,
        mint: {
            ...data.mint,
            supply: toBN(data.mint.supply),
        },
        actions: data.actions.map(action => {
            // Handle MintToCompressed action with recipients
            if ('mintToCompressed' in action && action.mintToCompressed) {
                return {
                    mintToCompressed: {
                        ...action.mintToCompressed,
                        recipients: action.mintToCompressed.recipients.map(r => ({
                            ...r,
                            amount: toBN(r.amount),
                        })),
                    },
                };
            }
            // Handle MintToCToken action
            if ('mintToCToken' in action && action.mintToCToken) {
                return {
                    mintToCToken: {
                        ...action.mintToCToken,
                        amount: toBN(action.mintToCToken.amount),
                    },
                };
            }
            return action;
        }),
    };

    const buffer = Buffer.alloc(10000); // Generous allocation
    const len = MintActionCompressedInstructionDataLayout.encode(
        encodableData,
        buffer,
    );

    return Buffer.concat([
        MINT_ACTION_DISCRIMINATOR,
        buffer.subarray(0, len),
    ]);
}

/**
 * Decode MintActionCompressedInstructionData from buffer
 *
 * @param buffer - The buffer to decode (including discriminator)
 * @returns Decoded instruction data
 */
export function decodeMintActionInstructionData(
    buffer: Buffer,
): MintActionCompressedInstructionData {
    return MintActionCompressedInstructionDataLayout.decode(
        buffer.subarray(MINT_ACTION_DISCRIMINATOR.length),
    ) as MintActionCompressedInstructionData;
}

