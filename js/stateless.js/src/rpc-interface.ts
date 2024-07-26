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
    nullable,
} from 'superstruct';
import type { Struct } from 'superstruct';
import {
    BN254,
    createBN254,
    CompressedProof,
    CompressedAccountWithMerkleContext,
    MerkleContextWithMerkleProof,
    bn,
    TokenData,
} from './state';
import { BN } from '@coral-xyz/anchor';

export interface LatestNonVotingSignatures {
    context: { slot: number };
    value: {
        items: {
            signature: string;
            slot: number;
            blockTime: number;
            error: string | null;
        }[];
    };
}

export interface LatestNonVotingSignaturesPaginated {
    context: { slot: number };
    value: {
        items: {
            signature: string;
            slot: number;
            blockTime: number;
        }[];
        cursor: string | null;
    };
}

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

export interface HexBatchInputsForProver {
    'input-compressed-accounts': HexInputsForProver[];
}

export interface HexInputsForProver {
    root: string;
    pathIndex: number;
    pathElements: string[];
    leaf: string;
}

// TODO: Rename Compressed -> ValidityProof
export type CompressedProofWithContext = {
    compressedProof: CompressedProof;
    roots: BN[];
    rootIndices: number[];
    leafIndices: number[];
    leaves: BN[];
    merkleTrees: PublicKey[];
    nullifierQueues: PublicKey[];
};

export interface GetCompressedTokenAccountsByOwnerOrDelegateOptions {
    mint?: PublicKey;
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
const ArrayFromString = coerce(instance(Array<number>), string(), value =>
    Array.from(new PublicKey(value).toBytes()),
);

/**
 * @internal
 */
const BN254FromString = coerce(instance(BN), string(), value => {
    return createBN254(value, 'base58');
});

const BNFromInt = coerce(instance(BN), number(), value => {
    // Check if the number is safe
    if (Number.isSafeInteger(value)) {
        return bn(value);
    } else {
        // Convert to string if the number is unsafe
        return bn(value.toString(), 10);
    }
});

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
    address: nullable(ArrayFromString),
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
    slotCreated: BNFromInt,
});

export const TokenDataResult = pick({
    mint: PublicKeyFromString,
    owner: PublicKeyFromString,
    amount: BNFromInt,
    delegate: nullable(PublicKeyFromString),
    state: string(),
});

/**
 * @internal
 */
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
export const LatestNonVotingSignaturesResult = pick({
    items: array(
        pick({
            signature: string(),
            slot: number(),
            blockTime: number(),
            error: nullable(string()),
        }),
    ),
});

/**
 * @internal
 */
export const LatestNonVotingSignaturesResultPaginated = pick({
    items: array(
        pick({
            signature: string(),
            slot: number(),
            blockTime: number(),
        }),
    ),
    cursor: nullable(string()),
});

/**
 * @internal
 */
export const MerkeProofResult = pick({
    hash: BN254FromString,
    leafIndex: number(),
    merkleTree: PublicKeyFromString,
    proof: array(BN254FromString),
    rootSeq: number(),
    root: BN254FromString,
});

/**
 * @internal
 */
export const NewAddressProofResult = pick({
    address: BN254FromString,
    nextIndex: number(),
    merkleTree: PublicKeyFromString,
    proof: array(BN254FromString), // this is: merkleProofHashedIndexedElementLeaf
    rootSeq: number(),
    root: BN254FromString,
    lowerRangeAddress: BN254FromString, // this is: leafLowerRangeValue.
    higherRangeAddress: BN254FromString, // this is: leafHigherRangeValue
    lowElementLeafIndex: number(), // this is: indexHashedIndexedElementLeaf
});

/**
 * @internal
 */
const CompressedProofResult = pick({
    a: array(number()),
    b: array(number()),
    c: array(number()),
});

/**
 * @internal
 */
export const ValidityProofResult = pick({
    compressedProof: CompressedProofResult,
    leafIndices: array(number()),
    leaves: array(BN254FromString),
    rootIndices: array(number()),
    roots: array(BN254FromString),
    merkleTrees: array(PublicKeyFromString),
    // TODO: enable nullifierQueues
    // nullifierQueues: array(PublicKeyFromString),
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
            blockTime: number(),
            signature: string(),
            slot: number(),
        }),
    ),
});

export const SignatureListWithCursorResult = pick({
    items: array(
        pick({
            blockTime: number(),
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
                optionalTokenData: nullable(TokenDataResult),
            }),
        ),
        openedAccounts: array(
            pick({
                account: CompressedAccountResult,
                optionalTokenData: nullable(TokenDataResult),
            }),
        ),
    }),
    /// TODO: add transaction struct
    /// https://github.com/solana-labs/solana/blob/27eff8408b7223bb3c4ab70523f8a8dca3ca6645/transaction-status/src/lib.rs#L1061
    transaction: any(),
});

export interface CompressionApiInterface {
    getCompressedAccount(
        address?: BN254,
        hash?: BN254,
    ): Promise<CompressedAccountWithMerkleContext | null>;

    getCompressedBalance(address?: BN254, hash?: BN254): Promise<BN | null>;

    getCompressedBalanceByOwner(owner: PublicKey): Promise<BN>;

    getCompressedAccountProof(
        hash: BN254,
    ): Promise<MerkleContextWithMerkleProof>;

    getMultipleCompressedAccounts(
        hashes: BN254[],
    ): Promise<CompressedAccountWithMerkleContext[]>;

    getMultipleCompressedAccountProofs(
        hashes: BN254[],
    ): Promise<MerkleContextWithMerkleProof[]>;

    getValidityProof(
        hashes: BN254[],
        newAddresses: BN254[],
    ): Promise<CompressedProofWithContext>;

    getValidityProofAndRpcContext(
        hashes: BN254[],
        newAddresses: BN254[],
    ): Promise<WithContext<CompressedProofWithContext>>;

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

    getTransactionWithCompressionInfo(
        signature: string,
    ): Promise<CompressedTransaction | null>;

    getCompressionSignaturesForAccount(
        hash: BN254,
    ): Promise<SignatureWithMetadata[]>;

    getCompressionSignaturesForAddress(
        address: PublicKey,
    ): Promise<SignatureWithMetadata[]>;

    getCompressionSignaturesForOwner(
        owner: PublicKey,
    ): Promise<SignatureWithMetadata[]>;

    getCompressionSignaturesForTokenOwner(
        owner: PublicKey,
    ): Promise<SignatureWithMetadata[]>;

    getLatestNonVotingSignatures(
        limit?: number,
    ): Promise<LatestNonVotingSignatures>;

    getLatestCompressionSignatures(
        cursor?: string,
        limit?: number,
    ): Promise<LatestNonVotingSignaturesPaginated>;

    getIndexerHealth(): Promise<string>;

    getIndexerSlot(): Promise<number>;
}
