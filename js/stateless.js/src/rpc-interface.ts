import { PublicKey, DataSizeFilter, MemcmpFilter } from '@solana/web3.js';

import {
    type as pick,
    number,
    string,
    array,
    literal,
    union,
    coerce,
    instance,
    create,
    unknown,
    any,
    boolean,
    nullable,
} from 'superstruct';
import type { Struct } from 'superstruct';
import {
    MerkleUpdateContext,
    BN254,
    createBN254,
    CompressedProof,
    CompressedAccountWithMerkleContext,
    MerkleContextWithMerkleProof,
    bn,
    TokenData,
} from './state';
import { BN } from '@coral-xyz/anchor';

export interface SignatureWithMetadata {
    blockTime: number;
    signature: string;
    slot: number;
}

export interface CompressedTransaction {
    compressionInfo: {
        closedAccounts: {
            account: CompressedAccountWithMerkleContext;
            maybeTokenData: TokenData | null;
        }[];
        openedAccounts: {
            account: CompressedAccountWithMerkleContext;
            maybeTokenData: TokenData | null;
        }[];
    };
    transaction: any;
}

export interface HexInputsForProver {
    roots: string[];
    inPathIndices: number[];
    inPathElements: string[][];
    leaves: string[];
}

// TODO: Rename Compressed -> ValidityProof
// TODO: consistent types
export type CompressedProofWithContext = {
    compressedProof: CompressedProof;
    roots: BN[];
    // for now we assume latest root = allLeaves.length
    rootIndices: number[];
    leafIndices: number[];
    leaves: BN[];
    merkleTrees: PublicKey[];
    nullifierQueues: PublicKey[];
};

export interface GetCompressedTokenAccountsByOwnerOrDelegateOptions {
    mint: PublicKey;
    cursor?: string;
    limit?: BN;
}

export type GetCompressedAccountsFilter = MemcmpFilter | DataSizeFilter;

export type GetCompressedAccountConfig = {
    encoding?: string;
};

export type GetCompressedAccountsConfig = {
    encoding?: string;
    filters?: GetCompressedAccountsFilter[];
};

export interface ParsedTokenAccount {
    compressedAccount: CompressedAccountWithMerkleContext;
    parsed: TokenData;
}

export type WithMerkleUpdateContext<T> = {
    /** merkle update context */
    context: MerkleUpdateContext | null;
    /** response value */
    value: T;
};

export type WithContext<T> = {
    /** context */
    context: {
        slot: number;
    };
    /** response value */
    value: T;
};

/**
 * @internal
 */
const PublicKeyFromString = coerce(
    instance(PublicKey),
    string(),
    value => new PublicKey(value),
);

/**
 * @internal
 */
// TODO: use a BN254 class here for the 1st parameter
const BN254FromString = coerce(instance(BN), string(), value => {
    return createBN254(value, 'base58');
});

const BNFromInt = coerce(instance(BN), number(), value => bn(value));
const BNFromBase10String = coerce(instance(BN), string(), value => bn(value));

/**
 * @internal
 */
const Base64EncodedCompressedAccountDataResult = coerce(
    string(),
    string(),
    value => (value === '' ? null : value),
);
/**
 * @internal
 */
export function createRpcResult<T, U>(result: Struct<T, U>) {
    return union([
        pick({
            jsonrpc: literal('2.0'),
            id: string(),
            result,
        }),
        pick({
            jsonrpc: literal('2.0'),
            id: string(),
            error: pick({
                code: unknown(),
                message: string(),
                data: nullable(any()),
            }),
        }),
    ]);
}

/**
 * @internal
 */
const UnknownRpcResult = createRpcResult(unknown());

/**
 * @internal
 */
export function jsonRpcResult<T, U>(schema: Struct<T, U>) {
    return coerce(createRpcResult(schema), UnknownRpcResult, value => {
        if ('error' in value) {
            return value;
        } else {
            return {
                ...value,
                result: create(value.result, schema),
            };
        }
    });
}

/**
 * @internal
 */
export function jsonRpcResultAndContext<T, U>(value: Struct<T, U>) {
    return jsonRpcResult(
        pick({
            context: pick({
                slot: number(),
            }),
            value,
        }),
    );
}

/**
 * @internal
 */
export const CompressedAccountResult = pick({
    address: nullable(PublicKeyFromString),
    hash: BN254FromString,
    data: nullable(
        pick({
            data: Base64EncodedCompressedAccountDataResult,
            dataHash: BN254FromString,
            discriminator: BNFromInt,
        }),
    ),
    lamports: BNFromInt,
    owner: PublicKeyFromString,
    leafIndex: number(),
    tree: PublicKeyFromString,
    seq: nullable(BNFromInt),
    slotUpdated: BNFromInt,
});

export const TokenDataResult = pick({
    mint: PublicKeyFromString,
    owner: PublicKeyFromString,
    amount: BNFromInt,
    delegate: nullable(PublicKeyFromString),
    delegatedAmount: BNFromInt,
    isNative: nullable(BNFromInt),
    state: string(),
});
/**
 * @internal
 */
/// TODO: update with remaining fields as added to token program
export const CompressedTokenAccountResult = pick({
    tokenData: TokenDataResult,
    account: CompressedAccountResult,
});

/**
 * @internal
 */
export const MultipleCompressedAccountsResult = pick({
    items: array(CompressedAccountResult),
});

/**
 * @internal
 */
export const CompressedAccountsByOwnerResult = pick({
    items: array(CompressedAccountResult),
    cursor: nullable(PublicKeyFromString),
});

/**
 * @internal
 */
export const CompressedTokenAccountsByOwnerOrDelegateResult = pick({
    items: array(CompressedTokenAccountResult),
    cursor: nullable(PublicKeyFromString),
});

/**
 * @internal
 */
export const SlotResult = number();

/**
 * @internal
 */
export const HealthResult = string();

/**
 * @internal
 */
export const MerkeProofResult = pick({
    hash: BN254FromString,
    leafIndex: number(),
    merkleTree: PublicKeyFromString,
    proof: array(BN254FromString),
    rootSeq: number(),
});

/**
 * @internal
 */
export const MultipleMerkleProofsResult = array(MerkeProofResult);

/**
 * @internal
 */
export const BalanceResult = pick({
    amount: BNFromInt,
});

export const NativeBalanceResult = BNFromInt;

export const TokenBalanceResult = pick({
    balance: BNFromInt,
    mint: PublicKeyFromString,
});

export const TokenBalanceListResult = pick({
    tokenBalances: array(TokenBalanceResult),
    cursor: nullable(PublicKeyFromString),
});

/// TODO: we need to add: tree, nullifierQueue, leafIndex, rootIndex
export const AccountProofResult = pick({
    hash: array(number()),
    root: array(number()),
    proof: array(array(number())),
});
export const toUnixTimestamp = (blockTime: string): number => {
    return new Date(blockTime).getTime();
};

export const SignatureListResult = pick({
    items: array(
        pick({
            blockTime: coerce(number(), string(), toUnixTimestamp),
            signature: string(),
            slot: number(),
        }),
    ),
});

export const SignatureListWithCursorResult = pick({
    items: array(
        pick({
            blockTime: coerce(number(), string(), toUnixTimestamp),
            signature: string(),
            slot: number(),
        }),
    ),
    cursor: nullable(PublicKeyFromString),
});

export const CompressedTransactionResult = pick({
    compressionInfo: pick({
        closedAccounts: array(
            pick({
                account: CompressedAccountResult,
                optionTokenData: nullable(TokenDataResult),
            }),
        ),
        openedAccounts: array(
            pick({
                account: CompressedAccountResult,
                optionTokenData: nullable(TokenDataResult),
            }),
        ),
    }),
    /// TODO: add transaction struct
    /// https://github.com/solana-labs/solana/blob/27eff8408b7223bb3c4ab70523f8a8dca3ca6645/transaction-status/src/lib.rs#L1061
    transaction: any(),
});

export interface CompressionApiInterface {
    /** Retrieve compressed account by hash or address */
    getCompressedAccount(
        hash: BN254,
    ): Promise<CompressedAccountWithMerkleContext | null>;

    /**
     * Retrieve compressed lamport balance of a compressed account by hash or
     * address.
     */
    getCompressedBalance(hash: BN254): Promise<BN | null>;

    /** Retrieve compressed lamport balance of an owner */
    getCompressedBalanceByOwner(owner: PublicKey): Promise<BN>;

    /** Retrieve merkle proof for a compressed account */
    getCompressedAccountProof(
        hash: BN254,
    ): Promise<MerkleContextWithMerkleProof>; // TODO: expose context slot

    /** Retrieve compressed account by hash or address */
    getMultipleCompressedAccounts(
        hashes: BN254[],
    ): Promise<CompressedAccountWithMerkleContext[]>;

    /** Retrieve multiple merkle proofs for compressed accounts */
    getMultipleCompressedAccountProofs(
        hashes: BN254[],
    ): Promise<MerkleContextWithMerkleProof[]>;

    /** Retrieve compressed accounts by owner */
    getCompressedAccountsByOwner(
        owner: PublicKey,
    ): Promise<CompressedAccountWithMerkleContext[]>;

    getCompressedTokenAccountsByOwner(
        publicKey: PublicKey,
        options: GetCompressedTokenAccountsByOwnerOrDelegateOptions,
    ): Promise<ParsedTokenAccount[]>;

    getCompressedTokenAccountsByDelegate(
        delegate: PublicKey,
        options: GetCompressedTokenAccountsByOwnerOrDelegateOptions,
    ): Promise<ParsedTokenAccount[]>;

    getCompressedTokenAccountBalance(hash: BN254): Promise<{ amount: BN }>;

    getCompressedTokenBalancesByOwner(
        publicKey: PublicKey,
        options: GetCompressedTokenAccountsByOwnerOrDelegateOptions,
    ): Promise<{ balance: BN; mint: PublicKey }[]>;

    getSignaturesForCompressedAccount(
        hash: BN254,
    ): Promise<SignatureWithMetadata[]>;

    getCompressedTransaction(
        signature: string,
    ): Promise<CompressedTransaction | null>;

    getSignaturesForAddress3(
        address: PublicKey,
    ): Promise<SignatureWithMetadata[]>;

    getSignaturesForOwner(owner: PublicKey): Promise<SignatureWithMetadata[]>;

    getSignaturesForTokenOwner(
        owner: PublicKey,
    ): Promise<SignatureWithMetadata[]>;

    /** Retrieve health status of the node */
    getHealth(): Promise<string>;

    /** Retrieve the current slot */
    getSlot(): Promise<number>;
    /** Receive validity Proof for n compressed accounts */
    getValidityProof(hashes: BN254[]): Promise<CompressedProofWithContext>;
}
