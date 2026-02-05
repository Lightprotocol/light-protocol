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
import { bn } from '@lightprotocol/stateless.js';

export const MINT_ACTION_DISCRIMINATOR = Buffer.from([103]);

export const RecipientLayout = struct([publicKey('recipient'), u64('amount')]);

export const MintToCompressedActionLayout = struct([
    u8('tokenAccountVersion'),
    vec(RecipientLayout, 'recipients'),
]);

export const UpdateAuthorityLayout = struct([
    option(publicKey(), 'newAuthority'),
]);

export const MintToCTokenActionLayout = struct([
    u8('accountIndex'),
    u64('amount'),
]);

export const UpdateMetadataFieldActionLayout = struct([
    u8('extensionIndex'),
    u8('fieldType'),
    vecU8('key'),
    vecU8('value'),
]);

export const UpdateMetadataAuthorityActionLayout = struct([
    u8('extensionIndex'),
    publicKey('newAuthority'),
]);

export const RemoveMetadataKeyActionLayout = struct([
    u8('extensionIndex'),
    vecU8('key'),
    u8('idempotent'),
]);

export const DecompressMintActionLayout = struct([
    u8('rentPayment'),
    u32('writeTopUp'),
]);

export const CompressAndCloseCMintActionLayout = struct([u8('idempotent')]);

export const ActionLayout = rustEnum([
    MintToCompressedActionLayout.replicate('mintToCompressed'),
    UpdateAuthorityLayout.replicate('updateMintAuthority'),
    UpdateAuthorityLayout.replicate('updateFreezeAuthority'),
    MintToCTokenActionLayout.replicate('mintToCToken'),
    UpdateMetadataFieldActionLayout.replicate('updateMetadataField'),
    UpdateMetadataAuthorityActionLayout.replicate('updateMetadataAuthority'),
    RemoveMetadataKeyActionLayout.replicate('removeMetadataKey'),
    DecompressMintActionLayout.replicate('decompressMint'),
    CompressAndCloseCMintActionLayout.replicate('compressAndCloseCMint'),
]);

export const CompressedProofLayout = struct([
    array(u8(), 32, 'a'),
    array(u8(), 64, 'b'),
    array(u8(), 32, 'c'),
]);

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

export const CreateMintLayout = struct([
    array(u8(), 4, 'readOnlyAddressTrees'),
    array(u16(), 4, 'readOnlyAddressTreeRootIndices'),
]);

export const AdditionalMetadataLayout = struct([vecU8('key'), vecU8('value')]);

export const TokenMetadataInstructionDataLayout = struct([
    option(publicKey(), 'updateAuthority'),
    vecU8('name'),
    vecU8('symbol'),
    vecU8('uri'),
    option(vec(AdditionalMetadataLayout), 'additionalMetadata'),
]);

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

export const CompressedMintMetadataLayout = struct([
    u8('version'),
    bool('cmintDecompressed'),
    publicKey('mint'),
    array(u8(), 32, 'mintSigner'),
    u8('bump'),
]);

export const MintInstructionDataLayout = struct([
    u64('supply'),
    u8('decimals'),
    CompressedMintMetadataLayout.replicate('metadata'),
    option(publicKey(), 'mintAuthority'),
    option(publicKey(), 'freezeAuthority'),
    option(vec(ExtensionInstructionDataLayout), 'extensions'),
]);

export const MintActionCompressedInstructionDataLayout = struct([
    u32('leafIndex'),
    bool('proveByIndex'),
    u16('rootIndex'),
    u16('maxTopUp'),
    option(CreateMintLayout, 'createMint'),
    vec(ActionLayout, 'actions'),
    option(CompressedProofLayout, 'proof'),
    option(CpiContextLayout, 'cpiContext'),
    option(MintInstructionDataLayout, 'mint'),
]);

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

export interface DecompressMintAction {
    rentPayment: number;
    writeTopUp: number;
}

export interface CompressAndCloseCMintAction {
    idempotent: number;
}

export type Action =
    | { mintToCompressed: MintToCompressedAction }
    | { updateMintAuthority: UpdateAuthority }
    | { updateFreezeAuthority: UpdateAuthority }
    | { mintToCToken: MintToCTokenAction }
    | { updateMetadataField: UpdateMetadataFieldAction }
    | { updateMetadataAuthority: UpdateMetadataAuthorityAction }
    | { removeMetadataKey: RemoveMetadataKeyAction }
    | { decompressMint: DecompressMintAction }
    | { compressAndCloseCMint: CompressAndCloseCMintAction };

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

export interface TokenMetadataLayoutData {
    updateAuthority: PublicKey | null;
    name: Buffer;
    symbol: Buffer;
    uri: Buffer;
    additionalMetadata: AdditionalMetadata[] | null;
}

export type ExtensionInstructionData = {
    tokenMetadata: TokenMetadataLayoutData;
};

export interface CompressedMintMetadata {
    version: number;
    cmintDecompressed: boolean;
    mint: PublicKey;
    mintSigner: number[];
    bump: number;
}

export interface MintLayoutData {
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
    maxTopUp: number;
    createMint: CreateMint | null;
    actions: Action[];
    proof: ValidityProof | null;
    cpiContext: CpiContext | null;
    mint: MintLayoutData | null;
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
    const convertedActions = data.actions.map(action => {
        if ('mintToCompressed' in action && action.mintToCompressed) {
            return {
                mintToCompressed: {
                    ...action.mintToCompressed,
                    recipients: action.mintToCompressed.recipients.map(r => ({
                        ...r,
                        amount: bn(r.amount.toString()),
                    })),
                },
            };
        }
        if ('mintToCToken' in action && action.mintToCToken) {
            return {
                mintToCToken: {
                    ...action.mintToCToken,
                    amount: bn(action.mintToCToken.amount.toString()),
                },
            };
        }
        return action;
    });

    const buffer = Buffer.alloc(10000);

    const encodableData = {
        ...data,
        actions: convertedActions,
        mint: data.mint
            ? {
                  ...data.mint,
                  supply: bn(data.mint.supply.toString()),
              }
            : null,
    };
    const len = MintActionCompressedInstructionDataLayout.encode(
        encodableData,
        buffer,
    );

    return Buffer.concat([MINT_ACTION_DISCRIMINATOR, buffer.subarray(0, len)]);
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
