/**
 * High-level transaction builders that wire load → select → proof → instruction.
 *
 * These bridge the gap between token-client (data loading) and token-sdk (instruction building).
 */

import { type Address, getAddressCodec } from '@solana/addresses';
import { AccountRole, type Instruction, type AccountMeta } from '@solana/instructions';

import type { LightIndexer } from './indexer.js';
import {
    loadTokenAccountsForTransfer,
    loadAllTokenAccounts,
    loadMintContext,
    getOutputTreeInfo,
    type InputTokenAccount,
    type LoadTokenAccountsOptions,
    type MintContext,
} from './load.js';

import {
    IndexerError,
    IndexerErrorCode,
    type ValidityProofWithContext,
} from './client/index.js';
import {
    createTransfer2Instruction,
    createWrapInstruction,
    createUnwrapInstruction,
    createCompressSpl,
    createDecompressSpl,
    createMintActionInstruction,
    createApproveInstruction,
    createMintToInstruction,
    createAssociatedTokenAccountInstruction,
    createAssociatedTokenAccountIdempotentInstruction,
} from './instructions/index.js';
import {
    TOKEN_ACCOUNT_VERSION_V2,
    LIGHT_TOKEN_CONFIG,
    LIGHT_TOKEN_RENT_SPONSOR,
    SPL_TOKEN_PROGRAM_ID,
} from './constants.js';
import {
    type SplInterfaceInfo,
    getSplInterfaceInfo,
    deriveMintAddress,
    deriveAssociatedTokenAddress,
} from './utils/index.js';
import { getMintDecimals, type QueryRpc } from './queries.js';
import type {
    MintAction,
    MintActionInstructionData,
    MintRecipient,
    CompressedProof,
    ExtensionInstructionData,
} from './codecs/index.js';

// ============================================================================
// SHARED TYPES
// ============================================================================

/**
 * Result of building a compressed transfer instruction with loaded account data.
 */
export interface BuildTransferResult {
    /** The transfer instruction to include in the transaction */
    instruction: Instruction;
    /** The input token accounts used */
    inputs: InputTokenAccount[];
    /** The validity proof for the inputs */
    proof: ValidityProofWithContext;
    /** Total amount available (may exceed requested amount; change goes back to sender) */
    totalInputAmount: bigint;
}

/**
 * Minimal RPC interface for builder operations.
 */
export interface BuilderRpc {
    getAccountInfo(
        address: Address,
        config?: { encoding: string },
    ): Promise<{ value: { owner: Address; data: unknown } | null }>;
}

/**
 * User-friendly metadata field type names.
 */
export type MetadataFieldType = 'name' | 'symbol' | 'uri' | 'custom';

/** Maps string field type to the on-chain numeric enum value. */
const FIELD_TYPE_MAP: Record<MetadataFieldType, number> = {
    name: 0,
    symbol: 1,
    uri: 2,
    custom: 3,
};

/**
 * User-friendly recipient param using Address instead of raw bytes.
 */
export interface MintRecipientParam {
    /** Recipient address */
    recipient: Address;
    /** Amount to mint */
    amount: bigint;
}

// ============================================================================
// INTERNAL HELPERS
// ============================================================================

/** Convert an Address to a 32-byte Uint8Array. */
function addressToBytes(addr: Address): Uint8Array {
    return new Uint8Array(getAddressCodec().encode(addr));
}

/** Convert MintRecipientParam[] to codec-level MintRecipient[]. */
function toCodecRecipients(params: MintRecipientParam[]): MintRecipient[] {
    return params.map((p) => ({
        recipient: addressToBytes(p.recipient),
        amount: p.amount,
    }));
}

function bytesToHexKey(hash: Uint8Array): string {
    return Array.from(hash, (b) => b.toString(16).padStart(2, '0')).join('');
}

// ============================================================================
// COMPRESSED TRANSFER
// ============================================================================

/**
 * Builds a compressed token transfer (Transfer2) instruction by loading accounts,
 * selecting inputs, fetching a validity proof, and creating the instruction.
 *
 * @param params - Transfer parameters
 * @returns The instruction, inputs, and proof
 */
export async function buildCompressedTransfer(params: {
    /** Light indexer client */
    indexer: LightIndexer;
    /** Token account owner (sender) */
    owner: Address;
    /** Token mint */
    mint: Address;
    /** Amount to transfer */
    amount: bigint;
    /** Recipient owner address */
    recipientOwner: Address;
    /** Fee payer address (signer, writable) */
    feePayer: Address;
    /** Maximum top-up amount for rent (optional) */
    maxTopUp?: number;
    /** Maximum number of input accounts (default: 4) */
    maxInputs?: number;
}): Promise<BuildTransferResult> {
    const options: LoadTokenAccountsOptions = {
        mint: params.mint,
        maxInputs: params.maxInputs,
    };

    // Load and select accounts, fetch proof
    const loaded = await loadTokenAccountsForTransfer(
        params.indexer,
        params.owner,
        params.amount,
        options,
    );
    if (loaded.inputs.length === 0) {
        throw new IndexerError(
            IndexerErrorCode.InvalidResponse,
            'No inputs were selected for transfer',
        );
    }

    const proofRootIndexByHash = new Map<string, number>();
    for (const proofInput of loaded.proof.accounts) {
        if (!(proofInput.hash instanceof Uint8Array) || proofInput.hash.length !== 32) {
            throw new IndexerError(
                IndexerErrorCode.InvalidResponse,
                `Invalid proof account hash: expected 32-byte Uint8Array, got ${proofInput.hash?.length ?? 'null'} bytes`,
            );
        }
        const key = bytesToHexKey(proofInput.hash);
        if (proofRootIndexByHash.has(key)) {
            throw new IndexerError(
                IndexerErrorCode.InvalidResponse,
                `Duplicate proof entry for input hash ${key}`,
            );
        }
        const rootIndex = proofInput.rootIndex.rootIndex;
        if (!Number.isInteger(rootIndex) || rootIndex < 0 || rootIndex > 65535) {
            throw new IndexerError(
                IndexerErrorCode.InvalidResponse,
                `Invalid rootIndex ${rootIndex} for input hash ${key}`,
            );
        }
        proofRootIndexByHash.set(key, rootIndex);
    }

    const packedAddressMap = new Map<string, number>();
    const packedAccounts: AccountMeta[] = [];

    function getOrAddPacked(addr: Address, role: AccountRole): number {
        const existing = packedAddressMap.get(addr as string);
        if (existing !== undefined) return existing;
        const idx = packedAccounts.length;
        packedAddressMap.set(addr as string, idx);
        packedAccounts.push({ address: addr, role });
        return idx;
    }

    // 1. Add merkle tree/queue pairs first
    for (const input of loaded.inputs) {
        getOrAddPacked(input.merkleContext.tree, AccountRole.WRITABLE);
        getOrAddPacked(input.merkleContext.queue, AccountRole.WRITABLE);
    }

    // 2. Output queue (rollover-aware)
    const outputTreeInfo = getOutputTreeInfo(
        loaded.inputs[0].tokenAccount.account.treeInfo,
    );
    const outputQueueIdx = getOrAddPacked(outputTreeInfo.queue, AccountRole.WRITABLE);

    // 3. Mint (readonly)
    const mintIdx = getOrAddPacked(params.mint, AccountRole.READONLY);

    // 4. Owner (readonly)
    const ownerIdx = getOrAddPacked(params.owner, AccountRole.READONLY);

    // 5. Recipient (readonly)
    const recipientIdx = getOrAddPacked(params.recipientOwner, AccountRole.READONLY);

    // Build input token data
    const inTokenData = loaded.inputs.map((input) => {
        const treeIdx = getOrAddPacked(input.merkleContext.tree, AccountRole.WRITABLE);
        const queueIdx = getOrAddPacked(input.merkleContext.queue, AccountRole.WRITABLE);

        const inputHashKey = bytesToHexKey(input.tokenAccount.account.hash);
        const rootIndex = proofRootIndexByHash.get(inputHashKey);
        if (rootIndex === undefined) {
            throw new IndexerError(
                IndexerErrorCode.InvalidResponse,
                `Missing proof account for selected input hash ${inputHashKey}`,
            );
        }

        const delegateAddress = input.tokenAccount.token.delegate;
        const hasDelegate = delegateAddress !== null;
        const delegateIdx = hasDelegate
            ? getOrAddPacked(delegateAddress, AccountRole.READONLY)
            : 0;

        return {
            owner: ownerIdx,
            amount: input.tokenAccount.token.amount,
            hasDelegate,
            delegate: delegateIdx,
            mint: mintIdx,
            version: TOKEN_ACCOUNT_VERSION_V2,
            merkleContext: {
                merkleTreePubkeyIndex: treeIdx,
                queuePubkeyIndex: queueIdx,
                leafIndex: input.merkleContext.leafIndex,
                proveByIndex: input.merkleContext.proveByIndex,
            },
            rootIndex,
        };
    });

    // Output token data
    const outTokenData = [
        {
            owner: recipientIdx,
            amount: params.amount,
            hasDelegate: false,
            delegate: 0,
            mint: mintIdx,
            version: TOKEN_ACCOUNT_VERSION_V2,
        },
    ];

    if (loaded.totalAmount > params.amount) {
        outTokenData.push({
            owner: ownerIdx,
            amount: loaded.totalAmount - params.amount,
            hasDelegate: false,
            delegate: 0,
            mint: mintIdx,
            version: TOKEN_ACCOUNT_VERSION_V2,
        });
    }

    const instruction = createTransfer2Instruction({
        feePayer: params.feePayer,
        packedAccounts,
        data: {
            withTransactionHash: false,
            withLamportsChangeAccountMerkleTreeIndex: false,
            lamportsChangeAccountMerkleTreeIndex: 0,
            lamportsChangeAccountOwnerIndex: ownerIdx,
            outputQueue: outputQueueIdx,
            maxTopUp: params.maxTopUp ?? 65535,
            cpiContext: null,
            compressions: null,
            proof: loaded.proof.proof,
            inTokenData,
            outTokenData,
            inLamports: null,
            outLamports: null,
            inTlv: null,
            outTlv: null,
        },
    });

    return {
        instruction,
        inputs: loaded.inputs,
        proof: loaded.proof,
        totalInputAmount: loaded.totalAmount,
    };
}

// ============================================================================
// DELEGATED TRANSFER
// ============================================================================

/**
 * Builds a Transfer2 instruction that sends from a delegated account.
 *
 * @param params - Transfer parameters with delegate authority
 * @returns The instruction, inputs, and proof
 */
export async function buildTransferDelegated(params: {
    /** Light indexer client */
    indexer: LightIndexer;
    /** Delegate authority (signer) */
    delegate: Address;
    /** Token account owner */
    owner: Address;
    /** Token mint */
    mint: Address;
    /** Amount to transfer */
    amount: bigint;
    /** Recipient owner address */
    recipientOwner: Address;
    /** Fee payer (signer, writable) */
    feePayer: Address;
    /** Maximum top-up */
    maxTopUp?: number;
    /** Maximum number of input accounts */
    maxInputs?: number;
}): Promise<BuildTransferResult> {
    const options: LoadTokenAccountsOptions = {
        mint: params.mint,
        maxInputs: params.maxInputs,
    };

    const loaded = await loadTokenAccountsForTransfer(
        params.indexer,
        params.owner,
        params.amount,
        options,
    );
    if (loaded.inputs.length === 0) {
        throw new IndexerError(
            IndexerErrorCode.InvalidResponse,
            'No inputs were selected for delegated transfer',
        );
    }

    const proofRootIndexByHash = new Map<string, number>();
    for (const proofInput of loaded.proof.accounts) {
        const key = bytesToHexKey(proofInput.hash);
        proofRootIndexByHash.set(key, proofInput.rootIndex.rootIndex);
    }

    const packedAccounts: AccountMeta[] = [];
    const packedMap = new Map<string, number>();

    function getOrAdd(addr: Address, role: AccountRole): number {
        const existing = packedMap.get(addr as string);
        if (existing !== undefined) return existing;
        const idx = packedAccounts.length;
        packedMap.set(addr as string, idx);
        packedAccounts.push({ address: addr, role });
        return idx;
    }

    for (const input of loaded.inputs) {
        getOrAdd(input.merkleContext.tree, AccountRole.WRITABLE);
        getOrAdd(input.merkleContext.queue, AccountRole.WRITABLE);
    }

    const outputTreeInfo = getOutputTreeInfo(
        loaded.inputs[0].tokenAccount.account.treeInfo,
    );
    const outputQueueIdx = getOrAdd(outputTreeInfo.queue, AccountRole.WRITABLE);
    const mintIdx = getOrAdd(params.mint, AccountRole.READONLY);
    const ownerIdx = getOrAdd(params.owner, AccountRole.READONLY);
    const delegateIdx = getOrAdd(params.delegate, AccountRole.READONLY);
    const recipientIdx = getOrAdd(params.recipientOwner, AccountRole.READONLY);

    const inTokenData = loaded.inputs.map((input) => {
        const treeIdx = getOrAdd(input.merkleContext.tree, AccountRole.WRITABLE);
        const queueIdx = getOrAdd(input.merkleContext.queue, AccountRole.WRITABLE);
        const inputHashKey = bytesToHexKey(input.tokenAccount.account.hash);
        const rootIndex = proofRootIndexByHash.get(inputHashKey) ?? 0;

        return {
            owner: ownerIdx,
            amount: input.tokenAccount.token.amount,
            hasDelegate: true,
            delegate: delegateIdx,
            mint: mintIdx,
            version: TOKEN_ACCOUNT_VERSION_V2,
            merkleContext: {
                merkleTreePubkeyIndex: treeIdx,
                queuePubkeyIndex: queueIdx,
                leafIndex: input.merkleContext.leafIndex,
                proveByIndex: input.merkleContext.proveByIndex,
            },
            rootIndex,
        };
    });

    const outTokenData = [
        {
            owner: recipientIdx,
            amount: params.amount,
            hasDelegate: false,
            delegate: 0,
            mint: mintIdx,
            version: TOKEN_ACCOUNT_VERSION_V2,
        },
    ];

    if (loaded.totalAmount > params.amount) {
        outTokenData.push({
            owner: ownerIdx,
            amount: loaded.totalAmount - params.amount,
            hasDelegate: false,
            delegate: 0,
            mint: mintIdx,
            version: TOKEN_ACCOUNT_VERSION_V2,
        });
    }

    const instruction = createTransfer2Instruction({
        feePayer: params.feePayer,
        packedAccounts,
        data: {
            withTransactionHash: false,
            withLamportsChangeAccountMerkleTreeIndex: false,
            lamportsChangeAccountMerkleTreeIndex: 0,
            lamportsChangeAccountOwnerIndex: ownerIdx,
            outputQueue: outputQueueIdx,
            maxTopUp: params.maxTopUp ?? 65535,
            cpiContext: null,
            compressions: null,
            proof: loaded.proof.proof,
            inTokenData,
            outTokenData,
            inLamports: null,
            outLamports: null,
            inTlv: null,
            outTlv: null,
        },
    });

    return {
        instruction,
        inputs: loaded.inputs,
        proof: loaded.proof,
        totalInputAmount: loaded.totalAmount,
    };
}

// ============================================================================
// WRAP / UNWRAP BUILDERS
// ============================================================================

/**
 * Builds a wrap instruction (SPL → Light Token).
 *
 * @param params - Wrap parameters
 * @returns The wrap instruction
 */
export async function buildWrap(params: {
    rpc: BuilderRpc;
    source: Address;
    destination: Address;
    owner: Address;
    mint: Address;
    amount: bigint;
    decimals?: number;
    tokenProgram?: Address;
    feePayer?: Address;
    splInterfaceInfo?: SplInterfaceInfo;
}): Promise<Instruction> {
    const tokenProgram = params.tokenProgram ?? SPL_TOKEN_PROGRAM_ID;
    const decimals =
        params.decimals ??
        (await getMintDecimals(params.rpc as unknown as QueryRpc, params.mint));

    const splInterfaceInfo =
        params.splInterfaceInfo ??
        (await getSplInterfaceInfo(params.rpc, params.mint, tokenProgram));

    return createWrapInstruction({
        source: params.source,
        destination: params.destination,
        owner: params.owner,
        mint: params.mint,
        amount: params.amount,
        splInterfaceInfo,
        decimals,
        feePayer: params.feePayer,
    });
}

/**
 * Builds an unwrap instruction (Light Token → SPL).
 *
 * @param params - Unwrap parameters
 * @returns The unwrap instruction
 */
export async function buildUnwrap(params: {
    rpc: BuilderRpc;
    source: Address;
    destination: Address;
    owner: Address;
    mint: Address;
    amount: bigint;
    decimals?: number;
    tokenProgram?: Address;
    feePayer?: Address;
    splInterfaceInfo?: SplInterfaceInfo;
}): Promise<Instruction> {
    const tokenProgram = params.tokenProgram ?? SPL_TOKEN_PROGRAM_ID;
    const decimals =
        params.decimals ??
        (await getMintDecimals(params.rpc as unknown as QueryRpc, params.mint));

    const splInterfaceInfo =
        params.splInterfaceInfo ??
        (await getSplInterfaceInfo(params.rpc, params.mint, tokenProgram));

    return createUnwrapInstruction({
        source: params.source,
        destination: params.destination,
        owner: params.owner,
        mint: params.mint,
        amount: params.amount,
        splInterfaceInfo,
        decimals,
        feePayer: params.feePayer,
    });
}

// ============================================================================
// COMPRESS / DECOMPRESS BUILDERS
// ============================================================================

/**
 * Builds a compress instruction (SPL → compressed token accounts).
 *
 * @param params - Compress parameters
 * @returns The Transfer2 instruction
 */
export async function buildCompress(params: {
    rpc: BuilderRpc;
    source: Address;
    owner: Address;
    mint: Address;
    amount: bigint;
    recipientOwner?: Address;
    decimals?: number;
    tokenProgram?: Address;
    outputQueue: Address;
    feePayer?: Address;
    splInterfaceInfo?: SplInterfaceInfo;
    maxTopUp?: number;
}): Promise<Instruction> {
    const payer = params.feePayer ?? params.owner;
    const recipientOwner = params.recipientOwner ?? params.owner;
    const tokenProgram = params.tokenProgram ?? SPL_TOKEN_PROGRAM_ID;
    const decimals =
        params.decimals ??
        (await getMintDecimals(params.rpc as unknown as QueryRpc, params.mint));

    const splInterfaceInfo =
        params.splInterfaceInfo ??
        (await getSplInterfaceInfo(params.rpc, params.mint, tokenProgram));

    const packedAccounts: AccountMeta[] = [];
    const packedMap = new Map<string, number>();

    function getOrAdd(addr: Address, role: AccountRole): number {
        const existing = packedMap.get(addr as string);
        if (existing !== undefined) return existing;
        const idx = packedAccounts.length;
        packedMap.set(addr as string, idx);
        packedAccounts.push({ address: addr, role });
        return idx;
    }

    const outputQueueIdx = getOrAdd(params.outputQueue, AccountRole.WRITABLE);
    const mintIdx = getOrAdd(params.mint, AccountRole.READONLY);
    const ownerIdx = getOrAdd(params.owner, AccountRole.READONLY_SIGNER);
    const sourceIdx = getOrAdd(params.source, AccountRole.WRITABLE);
    const poolIdx = getOrAdd(splInterfaceInfo.poolAddress, AccountRole.WRITABLE);
    getOrAdd(tokenProgram, AccountRole.READONLY);
    const recipientIdx =
        recipientOwner === params.owner
            ? ownerIdx
            : getOrAdd(recipientOwner, AccountRole.READONLY);

    const compressions = [
        createCompressSpl({
            amount: params.amount,
            mintIndex: mintIdx,
            sourceIndex: sourceIdx,
            authorityIndex: ownerIdx,
            poolAccountIndex: poolIdx,
            poolIndex: splInterfaceInfo.poolIndex,
            bump: splInterfaceInfo.bump,
            decimals,
        }),
    ];

    const outTokenData = [
        {
            owner: recipientIdx,
            amount: params.amount,
            hasDelegate: false,
            delegate: 0,
            mint: mintIdx,
            version: TOKEN_ACCOUNT_VERSION_V2,
        },
    ];

    return createTransfer2Instruction({
        feePayer: payer,
        packedAccounts,
        data: {
            withTransactionHash: false,
            withLamportsChangeAccountMerkleTreeIndex: false,
            lamportsChangeAccountMerkleTreeIndex: 0,
            lamportsChangeAccountOwnerIndex: ownerIdx,
            outputQueue: outputQueueIdx,
            maxTopUp: params.maxTopUp ?? 65535,
            cpiContext: null,
            compressions,
            proof: null,
            inTokenData: [],
            outTokenData,
            inLamports: null,
            outLamports: null,
            inTlv: null,
            outTlv: null,
        },
    });
}

/**
 * Builds a decompress instruction (compressed → SPL token account).
 *
 * @param params - Decompress parameters
 * @returns The Transfer2 instruction and loaded account info
 */
export async function buildDecompress(params: {
    rpc: BuilderRpc;
    indexer: LightIndexer;
    owner: Address;
    mint: Address;
    amount: bigint;
    destination: Address;
    decimals?: number;
    tokenProgram?: Address;
    feePayer?: Address;
    splInterfaceInfo?: SplInterfaceInfo;
    maxInputs?: number;
    maxTopUp?: number;
}): Promise<BuildTransferResult> {
    const payer = params.feePayer ?? params.owner;
    const tokenProgram = params.tokenProgram ?? SPL_TOKEN_PROGRAM_ID;
    const decimals =
        params.decimals ??
        (await getMintDecimals(params.rpc as unknown as QueryRpc, params.mint));

    const splInterfaceInfo =
        params.splInterfaceInfo ??
        (await getSplInterfaceInfo(params.rpc, params.mint, tokenProgram));

    // Load compressed token accounts
    const loaded = await loadTokenAccountsForTransfer(
        params.indexer,
        params.owner,
        params.amount,
        { mint: params.mint, maxInputs: params.maxInputs },
    );
    if (loaded.inputs.length === 0) {
        throw new IndexerError(
            IndexerErrorCode.InvalidResponse,
            'No compressed accounts found for decompress',
        );
    }

    const proofRootIndexByHash = new Map<string, number>();
    for (const proofInput of loaded.proof.accounts) {
        const key = bytesToHexKey(proofInput.hash);
        proofRootIndexByHash.set(key, proofInput.rootIndex.rootIndex);
    }

    const packedAccounts: AccountMeta[] = [];
    const packedMap = new Map<string, number>();

    function getOrAdd(addr: Address, role: AccountRole): number {
        const existing = packedMap.get(addr as string);
        if (existing !== undefined) return existing;
        const idx = packedAccounts.length;
        packedMap.set(addr as string, idx);
        packedAccounts.push({ address: addr, role });
        return idx;
    }

    // Merkle tree/queue pairs first
    for (const input of loaded.inputs) {
        getOrAdd(input.merkleContext.tree, AccountRole.WRITABLE);
        getOrAdd(input.merkleContext.queue, AccountRole.WRITABLE);
    }

    const outputTreeInfo = getOutputTreeInfo(
        loaded.inputs[0].tokenAccount.account.treeInfo,
    );
    const outputQueueIdx = getOrAdd(outputTreeInfo.queue, AccountRole.WRITABLE);
    const mintIdx = getOrAdd(params.mint, AccountRole.READONLY);
    const ownerIdx = getOrAdd(params.owner, AccountRole.READONLY);
    const destIdx = getOrAdd(params.destination, AccountRole.WRITABLE);
    const poolIdx = getOrAdd(splInterfaceInfo.poolAddress, AccountRole.WRITABLE);
    getOrAdd(tokenProgram, AccountRole.READONLY);

    const inTokenData = loaded.inputs.map((input) => {
        const treeIdx = getOrAdd(input.merkleContext.tree, AccountRole.WRITABLE);
        const queueIdx = getOrAdd(input.merkleContext.queue, AccountRole.WRITABLE);
        const inputHashKey = bytesToHexKey(input.tokenAccount.account.hash);
        const rootIndex = proofRootIndexByHash.get(inputHashKey) ?? 0;
        const delegateAddress = input.tokenAccount.token.delegate;
        const hasDelegate = delegateAddress !== null;
        const delegateIdx = hasDelegate
            ? getOrAdd(delegateAddress, AccountRole.READONLY)
            : 0;

        return {
            owner: ownerIdx,
            amount: input.tokenAccount.token.amount,
            hasDelegate,
            delegate: delegateIdx,
            mint: mintIdx,
            version: TOKEN_ACCOUNT_VERSION_V2,
            merkleContext: {
                merkleTreePubkeyIndex: treeIdx,
                queuePubkeyIndex: queueIdx,
                leafIndex: input.merkleContext.leafIndex,
                proveByIndex: input.merkleContext.proveByIndex,
            },
            rootIndex,
        };
    });

    const compressions = [
        createDecompressSpl({
            amount: params.amount,
            mintIndex: mintIdx,
            recipientIndex: destIdx,
            poolAccountIndex: poolIdx,
            poolIndex: splInterfaceInfo.poolIndex,
            bump: splInterfaceInfo.bump,
            decimals,
        }),
    ];

    const outTokenData =
        loaded.totalAmount > params.amount
            ? [
                  {
                      owner: ownerIdx,
                      amount: loaded.totalAmount - params.amount,
                      hasDelegate: false,
                      delegate: 0,
                      mint: mintIdx,
                      version: TOKEN_ACCOUNT_VERSION_V2,
                  },
              ]
            : [];

    const instruction = createTransfer2Instruction({
        feePayer: payer,
        packedAccounts,
        data: {
            withTransactionHash: false,
            withLamportsChangeAccountMerkleTreeIndex: false,
            lamportsChangeAccountMerkleTreeIndex: 0,
            lamportsChangeAccountOwnerIndex: ownerIdx,
            outputQueue: outputQueueIdx,
            maxTopUp: params.maxTopUp ?? 65535,
            cpiContext: null,
            compressions,
            proof: loaded.proof.proof,
            inTokenData,
            outTokenData,
            inLamports: null,
            outLamports: null,
            inTlv: null,
            outTlv: null,
        },
    });

    return {
        instruction,
        inputs: loaded.inputs,
        proof: loaded.proof,
        totalInputAmount: loaded.totalAmount,
    };
}

// ============================================================================
// MINT MANAGEMENT BUILDERS (Auto-resolving)
// ============================================================================

/**
 * Internal: Common params for mint actions that operate on existing mints.
 */
interface ExistingMintActionParams {
    authority: Address;
    feePayer: Address;
    mintSigner: Address;
    outOutputQueue: Address;
    merkleTree: Address;
    leafIndex: number;
    rootIndex: number;
    proveByIndex?: boolean;
    proof?: CompressedProof | null;
    maxTopUp?: number;
}

/**
 * Internal: Build MintActionInstructionData for existing mint operations.
 */
function buildExistingMintData(
    params: ExistingMintActionParams,
    actions: MintAction[],
): MintActionInstructionData {
    return {
        leafIndex: params.leafIndex,
        proveByIndex: params.proveByIndex ?? false,
        rootIndex: params.rootIndex,
        maxTopUp: params.maxTopUp ?? 65535,
        createMint: null,
        actions,
        proof: params.proof ?? null,
        cpiContext: null,
        mint: null,
    };
}

/**
 * Internal: Resolve mint context from either provided context or auto-fetch.
 */
async function resolveMintContext(
    indexer: LightIndexer | undefined,
    mintSigner: Address | undefined,
    mintContext?: MintContext,
): Promise<MintContext> {
    if (mintContext) return mintContext;
    if (!indexer || !mintSigner) {
        throw new Error(
            'Either mintContext or both indexer and mintSigner must be provided',
        );
    }
    return loadMintContext(indexer, mintSigner);
}

/**
 * Internal: Convert MintContext to ExistingMintActionParams.
 */
function mintContextToParams(
    ctx: MintContext,
    authority: Address,
    feePayer: Address,
    maxTopUp?: number,
): ExistingMintActionParams {
    return {
        authority,
        feePayer,
        mintSigner: ctx.mintSigner,
        outOutputQueue: ctx.outOutputQueue,
        merkleTree: ctx.merkleTree,
        leafIndex: ctx.leafIndex,
        rootIndex: ctx.rootIndex,
        proveByIndex: ctx.proveByIndex,
        proof: ctx.proof,
        maxTopUp,
    };
}

/**
 * Builds a CreateMint instruction via MintAction.
 *
 * @param params - Create mint parameters
 * @returns The MintAction instruction
 */
export async function buildCreateMint(params: {
    mintSigner: Address;
    authority: Address;
    feePayer: Address;
    outOutputQueue: Address;
    merkleTree: Address;
    decimals: number;
    supply?: bigint;
    mintAuthority: Address;
    freezeAuthority?: Address | null;
    extensions?: ExtensionInstructionData[] | null;
    addressTree?: Address;
    rootIndex?: number;
    proof?: CompressedProof | null;
    maxTopUp?: number;
    actions?: MintAction[];
}): Promise<Instruction> {
    const { address: mintAddress, bump } = await deriveMintAddress(params.mintSigner);
    const mintSignerBytes = addressToBytes(params.mintSigner);
    const mintAddressBytes = addressToBytes(mintAddress);
    const mintAuthorityBytes = addressToBytes(params.mintAuthority);
    const freezeAuthorityBytes =
        params.freezeAuthority != null
            ? addressToBytes(params.freezeAuthority)
            : null;

    const data: MintActionInstructionData = {
        leafIndex: 0,
        proveByIndex: false,
        rootIndex: params.rootIndex ?? 0,
        maxTopUp: params.maxTopUp ?? 65535,
        createMint: {
            readOnlyAddressTrees: new Uint8Array(4),
            readOnlyAddressTreeRootIndices: [0, 0, 0, 0],
        },
        actions: params.actions ?? [],
        proof: params.proof ?? null,
        cpiContext: null,
        mint: {
            supply: params.supply ?? 0n,
            decimals: params.decimals,
            metadata: {
                version: 0,
                mintDecompressed: false,
                mint: mintAddressBytes,
                mintSigner: mintSignerBytes,
                bump,
            },
            mintAuthority: mintAuthorityBytes,
            freezeAuthority: freezeAuthorityBytes,
            extensions: params.extensions ?? null,
        },
    };

    return createMintActionInstruction({
        mintSigner: params.mintSigner,
        authority: params.authority,
        feePayer: params.feePayer,
        outOutputQueue: params.outOutputQueue,
        merkleTree: params.merkleTree,
        data,
    });
}

/**
 * Builds an UpdateMintAuthority instruction via MintAction.
 * Auto-resolves merkle context when indexer + mint are provided.
 *
 * @param params - Parameters including the new authority
 * @returns The MintAction instruction
 */
export async function buildUpdateMintAuthority(params: {
    indexer: LightIndexer;
    mint: Address;
    authority: Address;
    feePayer: Address;
    newAuthority: Address | null;
    mintContext?: MintContext;
    maxTopUp?: number;
}): Promise<Instruction> {
    const ctx = await resolveMintContext(params.indexer, params.mint, params.mintContext);
    const action: MintAction = {
        type: 'UpdateMintAuthority',
        newAuthority: params.newAuthority ? addressToBytes(params.newAuthority) : null,
    };
    const resolved = mintContextToParams(ctx, params.authority, params.feePayer, params.maxTopUp);
    return createMintActionInstruction({
        mintSigner: resolved.mintSigner,
        authority: resolved.authority,
        feePayer: resolved.feePayer,
        outOutputQueue: resolved.outOutputQueue,
        merkleTree: resolved.merkleTree,
        data: buildExistingMintData(resolved, [action]),
    });
}

/**
 * Builds an UpdateFreezeAuthority instruction via MintAction.
 * Auto-resolves merkle context when indexer + mint are provided.
 *
 * @param params - Parameters including the new freeze authority
 * @returns The MintAction instruction
 */
export async function buildUpdateFreezeAuthority(params: {
    indexer: LightIndexer;
    mint: Address;
    authority: Address;
    feePayer: Address;
    newAuthority: Address | null;
    mintContext?: MintContext;
    maxTopUp?: number;
}): Promise<Instruction> {
    const ctx = await resolveMintContext(params.indexer, params.mint, params.mintContext);
    const action: MintAction = {
        type: 'UpdateFreezeAuthority',
        newAuthority: params.newAuthority ? addressToBytes(params.newAuthority) : null,
    };
    const resolved = mintContextToParams(ctx, params.authority, params.feePayer, params.maxTopUp);
    return createMintActionInstruction({
        mintSigner: resolved.mintSigner,
        authority: resolved.authority,
        feePayer: resolved.feePayer,
        outOutputQueue: resolved.outOutputQueue,
        merkleTree: resolved.merkleTree,
        data: buildExistingMintData(resolved, [action]),
    });
}

/**
 * Builds an UpdateMetadataField instruction via MintAction.
 * Auto-resolves merkle context and extensionIndex.
 *
 * @param params - Parameters including field type and value
 * @returns The MintAction instruction
 */
export async function buildUpdateMetadataField(params: {
    indexer: LightIndexer;
    mint: Address;
    authority: Address;
    feePayer: Address;
    fieldType: MetadataFieldType;
    value: string;
    customKey?: string;
    extensionIndex?: number;
    mintContext?: MintContext;
    maxTopUp?: number;
}): Promise<Instruction> {
    const ctx = await resolveMintContext(params.indexer, params.mint, params.mintContext);
    const extensionIndex =
        params.extensionIndex ?? Math.max(0, ctx.metadataExtensionIndex);
    const fieldTypeNum = FIELD_TYPE_MAP[params.fieldType];
    const encoder = new TextEncoder();

    const action: MintAction = {
        type: 'UpdateMetadataField',
        extensionIndex,
        fieldType: fieldTypeNum,
        key:
            params.fieldType === 'custom' && params.customKey
                ? encoder.encode(params.customKey)
                : new Uint8Array(0),
        value: encoder.encode(params.value),
    };

    const resolved = mintContextToParams(ctx, params.authority, params.feePayer, params.maxTopUp);
    return createMintActionInstruction({
        mintSigner: resolved.mintSigner,
        authority: resolved.authority,
        feePayer: resolved.feePayer,
        outOutputQueue: resolved.outOutputQueue,
        merkleTree: resolved.merkleTree,
        data: buildExistingMintData(resolved, [action]),
    });
}

/**
 * Builds an UpdateMetadataAuthority instruction via MintAction.
 * Auto-resolves merkle context and extensionIndex.
 *
 * @param params - Parameters including the new metadata authority
 * @returns The MintAction instruction
 */
export async function buildUpdateMetadataAuthority(params: {
    indexer: LightIndexer;
    mint: Address;
    authority: Address;
    feePayer: Address;
    newAuthority: Address;
    extensionIndex?: number;
    mintContext?: MintContext;
    maxTopUp?: number;
}): Promise<Instruction> {
    const ctx = await resolveMintContext(params.indexer, params.mint, params.mintContext);
    const extensionIndex =
        params.extensionIndex ?? Math.max(0, ctx.metadataExtensionIndex);

    const action: MintAction = {
        type: 'UpdateMetadataAuthority',
        extensionIndex,
        newAuthority: addressToBytes(params.newAuthority),
    };

    const resolved = mintContextToParams(ctx, params.authority, params.feePayer, params.maxTopUp);
    return createMintActionInstruction({
        mintSigner: resolved.mintSigner,
        authority: resolved.authority,
        feePayer: resolved.feePayer,
        outOutputQueue: resolved.outOutputQueue,
        merkleTree: resolved.merkleTree,
        data: buildExistingMintData(resolved, [action]),
    });
}

/**
 * Builds a RemoveMetadataKey instruction via MintAction.
 * Auto-resolves merkle context and extensionIndex.
 *
 * @param params - Parameters including the key to remove
 * @returns The MintAction instruction
 */
export async function buildRemoveMetadataKey(params: {
    indexer: LightIndexer;
    mint: Address;
    authority: Address;
    feePayer: Address;
    key: string;
    idempotent?: boolean;
    extensionIndex?: number;
    mintContext?: MintContext;
    maxTopUp?: number;
}): Promise<Instruction> {
    const ctx = await resolveMintContext(params.indexer, params.mint, params.mintContext);
    const extensionIndex =
        params.extensionIndex ?? Math.max(0, ctx.metadataExtensionIndex);

    const action: MintAction = {
        type: 'RemoveMetadataKey',
        extensionIndex,
        key: new TextEncoder().encode(params.key),
        idempotent: params.idempotent ? 1 : 0,
    };

    const resolved = mintContextToParams(ctx, params.authority, params.feePayer, params.maxTopUp);
    return createMintActionInstruction({
        mintSigner: resolved.mintSigner,
        authority: resolved.authority,
        feePayer: resolved.feePayer,
        outOutputQueue: resolved.outOutputQueue,
        merkleTree: resolved.merkleTree,
        data: buildExistingMintData(resolved, [action]),
    });
}

// ============================================================================
// MINT TO BUILDERS
// ============================================================================

/**
 * Builds a MintToCompressed instruction via MintAction.
 * Auto-resolves merkle context. Uses Address-based recipients.
 *
 * @param params - Parameters including recipients
 * @returns The MintAction instruction
 */
export async function buildMintToCompressed(params: {
    indexer: LightIndexer;
    mint: Address;
    authority: Address;
    feePayer: Address;
    recipients: MintRecipientParam[];
    tokenAccountVersion?: number;
    mintContext?: MintContext;
    maxTopUp?: number;
}): Promise<Instruction> {
    const ctx = await resolveMintContext(params.indexer, params.mint, params.mintContext);
    const action: MintAction = {
        type: 'MintToCompressed',
        tokenAccountVersion: params.tokenAccountVersion ?? TOKEN_ACCOUNT_VERSION_V2,
        recipients: toCodecRecipients(params.recipients),
    };

    const resolved = mintContextToParams(ctx, params.authority, params.feePayer, params.maxTopUp);
    return createMintActionInstruction({
        mintSigner: resolved.mintSigner,
        authority: resolved.authority,
        feePayer: resolved.feePayer,
        outOutputQueue: resolved.outOutputQueue,
        merkleTree: resolved.merkleTree,
        data: buildExistingMintData(resolved, [action]),
    });
}

/**
 * Builds a MintTo instruction via MintAction (to an on-chain token account).
 * Auto-resolves merkle context. The user provides tokenAccount Address.
 *
 * @param params - Parameters including destination token account
 * @returns The MintAction instruction
 */
export async function buildMintToInterface(params: {
    indexer: LightIndexer;
    mint: Address;
    authority: Address;
    feePayer: Address;
    tokenAccount: Address;
    amount: bigint;
    mintContext?: MintContext;
    maxTopUp?: number;
}): Promise<Instruction> {
    const ctx = await resolveMintContext(params.indexer, params.mint, params.mintContext);
    const packedAccounts: AccountMeta[] = [
        { address: params.tokenAccount, role: AccountRole.WRITABLE },
    ];

    const action: MintAction = {
        type: 'MintTo',
        accountIndex: 0,
        amount: params.amount,
    };

    const resolved = mintContextToParams(ctx, params.authority, params.feePayer, params.maxTopUp);
    return createMintActionInstruction({
        mintSigner: resolved.mintSigner,
        authority: resolved.authority,
        feePayer: resolved.feePayer,
        outOutputQueue: resolved.outOutputQueue,
        merkleTree: resolved.merkleTree,
        data: buildExistingMintData(resolved, [action]),
        packedAccounts,
    });
}

/**
 * Builds a DecompressMint instruction via MintAction.
 * Auto-resolves merkle context.
 *
 * @param params - Parameters for decompress mint
 * @returns The MintAction instruction
 */
export async function buildDecompressMint(params: {
    indexer: LightIndexer;
    mint: Address;
    authority: Address;
    feePayer: Address;
    rentPayment?: number;
    writeTopUp?: number;
    compressibleConfig?: Address;
    cmint?: Address;
    rentSponsor?: Address;
    mintContext?: MintContext;
    maxTopUp?: number;
}): Promise<Instruction> {
    const ctx = await resolveMintContext(params.indexer, params.mint, params.mintContext);
    const action: MintAction = {
        type: 'DecompressMint',
        rentPayment: params.rentPayment ?? 2,
        writeTopUp: params.writeTopUp ?? 0,
    };

    const resolved = mintContextToParams(ctx, params.authority, params.feePayer, params.maxTopUp);
    return createMintActionInstruction({
        mintSigner: resolved.mintSigner,
        authority: resolved.authority,
        feePayer: resolved.feePayer,
        outOutputQueue: resolved.outOutputQueue,
        merkleTree: resolved.merkleTree,
        data: buildExistingMintData(resolved, [action]),
        compressibleConfig: params.compressibleConfig ?? LIGHT_TOKEN_CONFIG,
        cmint: params.cmint,
        rentSponsor: params.rentSponsor ?? LIGHT_TOKEN_RENT_SPONSOR,
    });
}

// ============================================================================
// APPROVE AND MINT TO
// ============================================================================

/**
 * Builds approve + mint-to instructions for a single transaction.
 *
 * @param params - Approve and mint parameters
 * @returns Array of two instructions [approve, mintTo]
 */
export function buildApproveAndMintTo(params: {
    tokenAccount: Address;
    mint: Address;
    delegate: Address;
    owner: Address;
    mintAuthority: Address;
    approveAmount: bigint;
    mintAmount: bigint;
    feePayer?: Address;
    maxTopUp?: number;
}): Instruction[] {
    const approveIx = createApproveInstruction({
        tokenAccount: params.tokenAccount,
        delegate: params.delegate,
        owner: params.owner,
        amount: params.approveAmount,
        maxTopUp: params.maxTopUp,
    });

    const mintToIx = createMintToInstruction({
        mint: params.mint,
        tokenAccount: params.tokenAccount,
        mintAuthority: params.mintAuthority,
        amount: params.mintAmount,
        maxTopUp: params.maxTopUp,
        feePayer: params.feePayer,
    });

    return [approveIx, mintToIx];
}

// ============================================================================
// COMPRESS SPL TOKEN ACCOUNT
// ============================================================================

/**
 * Builds a Transfer2 instruction to compress an SPL token account.
 *
 * @param params - Compress SPL token account parameters
 * @returns The Transfer2 instruction
 */
export async function buildCompressSplTokenAccount(params: {
    rpc: BuilderRpc;
    source: Address;
    owner: Address;
    mint: Address;
    amount: bigint;
    decimals?: number;
    tokenProgram?: Address;
    outputQueue: Address;
    feePayer?: Address;
    splInterfaceInfo?: SplInterfaceInfo;
    maxTopUp?: number;
}): Promise<Instruction> {
    return buildCompress({
        rpc: params.rpc,
        source: params.source,
        owner: params.owner,
        mint: params.mint,
        amount: params.amount,
        decimals: params.decimals,
        tokenProgram: params.tokenProgram,
        outputQueue: params.outputQueue,
        feePayer: params.feePayer,
        splInterfaceInfo: params.splInterfaceInfo,
        maxTopUp: params.maxTopUp,
    });
}

// ============================================================================
// TRANSFER INTERFACE
// ============================================================================

/**
 * Builds a transfer via the unified interface.
 *
 * @param params - Transfer interface parameters
 * @returns Array of instructions to execute
 */
export async function buildTransferInterface(params: {
    indexer: LightIndexer;
    owner: Address;
    mint: Address;
    amount: bigint;
    recipientOwner: Address;
    feePayer: Address;
    maxTopUp?: number;
    maxInputs?: number;
}): Promise<{ instructions: Instruction[]; transferResult: BuildTransferResult }> {
    const result = await buildCompressedTransfer({
        indexer: params.indexer,
        owner: params.owner,
        mint: params.mint,
        amount: params.amount,
        recipientOwner: params.recipientOwner,
        feePayer: params.feePayer,
        maxTopUp: params.maxTopUp,
        maxInputs: params.maxInputs,
    });

    return {
        instructions: [result.instruction],
        transferResult: result,
    };
}

// ============================================================================
// LOAD ATA
// ============================================================================

/**
 * Builds instructions to load a Light Token ATA from compressed (cold) balances.
 *
 * @param params - Load ATA parameters
 * @returns Array of decompress instructions (may be empty)
 */
export async function buildLoadAta(params: {
    rpc: BuilderRpc;
    indexer: LightIndexer;
    owner: Address;
    mint: Address;
    destination: Address;
    decimals?: number;
    tokenProgram?: Address;
    feePayer?: Address;
    splInterfaceInfo?: SplInterfaceInfo;
    maxInputsPerInstruction?: number;
}): Promise<Instruction[]> {
    const maxInputs = params.maxInputsPerInstruction ?? 4;
    const payer = params.feePayer ?? params.owner;

    const allAccounts = await loadAllTokenAccounts(
        params.indexer,
        params.owner,
        { mint: params.mint },
    );

    if (allAccounts.length === 0) {
        return [];
    }

    const totalColdBalance = allAccounts.reduce(
        (sum, acc) => sum + acc.token.amount,
        0n,
    );
    if (totalColdBalance === 0n) {
        return [];
    }

    const result = await buildDecompress({
        rpc: params.rpc,
        indexer: params.indexer,
        owner: params.owner,
        mint: params.mint,
        amount: totalColdBalance,
        destination: params.destination,
        decimals: params.decimals,
        tokenProgram: params.tokenProgram,
        feePayer: payer,
        splInterfaceInfo: params.splInterfaceInfo,
        maxInputs,
    });

    return [result.instruction];
}

// ============================================================================
// CREATE ATA BUILDERS (NEW)
// ============================================================================

/**
 * Builds a createAssociatedTokenAccount instruction.
 * Derives the ATA address automatically.
 *
 * @param params - Owner, mint, feePayer
 * @returns Instruction, derived ATA address, and bump
 */
export async function buildCreateAta(params: {
    owner: Address;
    mint: Address;
    feePayer: Address;
}): Promise<{ instruction: Instruction; ata: Address; bump: number }> {
    const { address: ata, bump, instruction } =
        await createAssociatedTokenAccountInstruction({
            payer: params.feePayer,
            owner: params.owner,
            mint: params.mint,
        });
    return { instruction, ata: ata, bump };
}

/**
 * Builds an idempotent createAssociatedTokenAccount instruction.
 * Derives the ATA address automatically.
 *
 * @param params - Owner, mint, feePayer
 * @returns Instruction, derived ATA address, and bump
 */
export async function buildCreateAtaIdempotent(params: {
    owner: Address;
    mint: Address;
    feePayer: Address;
}): Promise<{ instruction: Instruction; ata: Address; bump: number }> {
    const { address: ata, bump, instruction } =
        await createAssociatedTokenAccountIdempotentInstruction({
            payer: params.feePayer,
            owner: params.owner,
            mint: params.mint,
        });
    return { instruction, ata: ata, bump };
}

// ============================================================================
// GET OR CREATE ATA (NEW)
// ============================================================================

/**
 * Builds instructions to ensure an ATA exists and load cold balances.
 *
 * Returns instructions to:
 * 1. Create the ATA if it doesn't exist on-chain (idempotent)
 * 2. Decompress cold compressed token balances into the ATA
 *
 * @param params - Get or create ATA parameters
 * @returns Instructions, ATA address, and balance info
 */
export async function buildGetOrCreateAta(params: {
    rpc: BuilderRpc;
    indexer: LightIndexer;
    owner: Address;
    mint: Address;
    feePayer: Address;
    tokenProgram?: Address;
    decimals?: number;
    splInterfaceInfo?: SplInterfaceInfo;
}): Promise<{
    instructions: Instruction[];
    ata: Address;
    hotBalance: bigint;
    coldBalance: bigint;
    totalBalance: bigint;
}> {
    const instructions: Instruction[] = [];

    // 1. Derive ATA address
    const { address: ata } = await deriveAssociatedTokenAddress(
        params.owner,
        params.mint,
    );

    // 2. Check if ATA exists on-chain
    let hotBalance = 0n;
    try {
        const info = await params.rpc.getAccountInfo(ata, { encoding: 'base64' });
        if (!info.value) {
            // ATA doesn't exist — add idempotent create instruction
            const { instruction } = await buildCreateAtaIdempotent({
                owner: params.owner,
                mint: params.mint,
                feePayer: params.feePayer,
            });
            instructions.push(instruction);
        } else {
            // Parse hot balance from on-chain data
            const data = info.value.data;
            if (data && typeof data === 'object' && Array.isArray(data)) {
                const bytes = Uint8Array.from(
                    atob(data[0] as string),
                    (c) => c.charCodeAt(0),
                );
                if (bytes.length >= 72) {
                    const view = new DataView(
                        bytes.buffer,
                        bytes.byteOffset,
                        bytes.byteLength,
                    );
                    hotBalance = view.getBigUint64(64, true);
                }
            }
        }
    } catch {
        // If getAccountInfo fails, assume ATA doesn't exist
        const { instruction } = await buildCreateAtaIdempotent({
            owner: params.owner,
            mint: params.mint,
            feePayer: params.feePayer,
        });
        instructions.push(instruction);
    }

    // 3. Load compressed accounts
    const coldAccounts = await loadAllTokenAccounts(
        params.indexer,
        params.owner,
        { mint: params.mint },
    );
    const coldBalance = coldAccounts.reduce(
        (sum, acc) => sum + acc.token.amount,
        0n,
    );

    // 4. If cold balance exists, add decompress instructions
    if (coldBalance > 0n) {
        const decompressResult = await buildDecompress({
            rpc: params.rpc,
            indexer: params.indexer,
            owner: params.owner,
            mint: params.mint,
            amount: coldBalance,
            destination: ata,
            decimals: params.decimals,
            tokenProgram: params.tokenProgram,
            feePayer: params.feePayer,
            splInterfaceInfo: params.splInterfaceInfo,
        });
        instructions.push(decompressResult.instruction);
    }

    return {
        instructions,
        ata,
        hotBalance,
        coldBalance,
        totalBalance: hotBalance + coldBalance,
    };
}

// ============================================================================
// DECOMPRESS INTERFACE (NEW)
// ============================================================================

/**
 * Builds decompress instructions with auto-derived ATA creation.
 *
 * When destination is omitted, derives the Light Token ATA for owner+mint
 * and creates it idempotently if needed.
 *
 * @param params - Decompress interface parameters
 * @returns Instructions array and destination address
 */
export async function buildDecompressInterface(params: {
    rpc: BuilderRpc;
    indexer: LightIndexer;
    owner: Address;
    mint: Address;
    amount?: bigint;
    destination?: Address;
    destinationOwner?: Address;
    feePayer?: Address;
    tokenProgram?: Address;
    decimals?: number;
    splInterfaceInfo?: SplInterfaceInfo;
}): Promise<{ instructions: Instruction[]; destination: Address }> {
    const instructions: Instruction[] = [];
    const payer = params.feePayer ?? params.owner;
    const destOwner = params.destinationOwner ?? params.owner;

    // Resolve destination
    let destination: Address;
    if (params.destination) {
        destination = params.destination;
    } else {
        // Derive ATA and create idempotently
        const { address: ata } = await deriveAssociatedTokenAddress(
            destOwner,
            params.mint,
        );
        destination = ata;

        // Check if it exists
        try {
            const info = await params.rpc.getAccountInfo(ata, {
                encoding: 'base64',
            });
            if (!info.value) {
                const { instruction } = await buildCreateAtaIdempotent({
                    owner: destOwner,
                    mint: params.mint,
                    feePayer: payer,
                });
                instructions.push(instruction);
            }
        } catch {
            const { instruction } = await buildCreateAtaIdempotent({
                owner: destOwner,
                mint: params.mint,
                feePayer: payer,
            });
            instructions.push(instruction);
        }
    }

    // Determine amount
    let decompressAmount: bigint;
    if (params.amount !== undefined) {
        decompressAmount = params.amount;
    } else {
        // Load all compressed accounts to decompress entire balance
        const accounts = await loadAllTokenAccounts(
            params.indexer,
            params.owner,
            { mint: params.mint },
        );
        decompressAmount = accounts.reduce(
            (sum, acc) => sum + acc.token.amount,
            0n,
        );
    }

    if (decompressAmount > 0n) {
        const result = await buildDecompress({
            rpc: params.rpc,
            indexer: params.indexer,
            owner: params.owner,
            mint: params.mint,
            amount: decompressAmount,
            destination,
            decimals: params.decimals,
            tokenProgram: params.tokenProgram,
            feePayer: payer,
            splInterfaceInfo: params.splInterfaceInfo,
        });
        instructions.push(result.instruction);
    }

    return { instructions, destination };
}
