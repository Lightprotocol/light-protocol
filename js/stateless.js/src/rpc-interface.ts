import { PublicKey, MemcmpFilter, DataSlice } from '@solana/web3.js';
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
    Struct,
    boolean,
    optional,
} from 'superstruct';
import {
    BN254,
    createBN254,
    ValidityProof,
    CompressedAccountWithMerkleContext,
    MerkleContextWithMerkleProof,
    bn,
    TokenData,
    TreeInfo,
    AddressTreeInfo,
    CompressedProof,
} from './state';
import BN from 'bn.js';

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

export interface GetCompressedAccountsByOwnerConfig {
    filters?: GetCompressedAccountsFilter[];
    dataSlice?: DataSlice;
    cursor?: string;
    limit?: BN;
}

export interface CompressedMintTokenHolders {
    balance: BN;
    owner: PublicKey;
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

/**
 * Account hash and associated state tree info.
 */
export interface HashWithTreeInfo {
    /**
     * Account hash.
     */
    hash: BN254;
    /**
     * State tree info.
     */
    stateTreeInfo: TreeInfo;
}

/**
 * Address and associated address tree info.
 */
export interface AddressWithTreeInfo {
    /**
     * Address.
     */
    address: BN254;
    /**
     * Address tree info.
     */
    addressTreeInfo: AddressTreeInfo;
}

export interface HashWithTree {
    hash: BN254;
    tree: PublicKey;
    queue: PublicKey;
}

export interface AddressWithTree {
    address: BN254;
    tree: PublicKey;
    queue: PublicKey;
}

export interface AddressWithTreeInfo {
    address: BN254;
    treeInfo: AddressTreeInfo;
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
        preTokenBalances?: {
            owner: PublicKey;
            mint: PublicKey;
            amount: BN;
        }[];
        postTokenBalances?: {
            owner: PublicKey;
            mint: PublicKey;
            amount: BN;
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

/**
 * Validity proof with context.
 *
 * You can request proofs via `rpc.getValidityProof` or
 * `rpc.getValidityProofV0`.
 */
export type ValidityProofWithContext = {
    /**
     * Validity proof.
     */
    compressedProof: ValidityProof | null;
    /**
     * Roots.
     */
    roots: BN[];
    /**
     * Root indices.
     */
    rootIndices: number[];
    /**
     * Leaf indices.
     */
    leafIndices: number[];
    /**
     * Leaves.
     */
    leaves: BN[];
    /**
     * Tree infos.
     */
    treeInfos: TreeInfo[];
    /**
     * Whether to prove by indices.
     */
    proveByIndices: boolean[];
};

/**
 * @deprecated use {@link ValidityProofWithContext} instead
 */
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
export type TokenBalance = { balance: BN; mint: PublicKey };

/**
 * **Cursor** is a unique identifier for a page of results by which the next page can be fetched.
 *
 * **Limit** is the maximum number of results to return per page.
 */
export interface PaginatedOptions {
    cursor?: string;
    limit?: BN;
}

/**
 * Note, DataSizeFilter is currently not available.
 */
export type GetCompressedAccountsFilter = MemcmpFilter; // | DataSizeFilter;

export type GetCompressedAccountConfig = {
    encoding?: string;
};

export type GetCompressedAccountsConfig = {
    dataSlice: DataSlice;
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

export type WithCursor<T> = {
    /** context */
    cursor: string | null;
    /** response value */
    items: T;
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

/**
 * @internal
 * Coerce number (0/1) or boolean to boolean
 */
const BooleanFromNumberOrBoolean = coerce(
    boolean(),
    union([number(), boolean()]),
    value => Boolean(value),
);

/**
 *
 * @internal
 * expects bigints to be supplied as strings.
 */
const BNFromStringOrNumber = coerce(
    instance(BN),
    union([string(), number()]),
    value => {
        if (typeof value === 'number') {
            if (!Number.isSafeInteger(value)) {
                throw new Error(`Unsafe integer. Precision loss: ${value}`);
            }
            return bn(value); // Safe number → BN
        }
        return bn(value, 10); // String → BN
    },
);

/**
 *
 * @internal
 */
const Base64EncodedCompressedAccountDataResult = string();
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
    ]) as Struct<RpcResult<T>, null>;
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
            return value as RpcResultError;
        } else {
            return {
                ...value,
                result: create(value.result, schema),
            } as RpcResultSuccess<T>;
        }
    }) as Struct<RpcResult<T>, null>;
}

// Add this type for the context wrapper
export type WithRpcContext<T> = {
    context: {
        slot: number;
    };
    value: T;
};

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
    ) as Struct<RpcResult<WithRpcContext<T>>, null>;
}

const NextTreeInfoResultV2 = pick({
    treeType: number(),
    tree: PublicKeyFromString,
    queue: PublicKeyFromString,
    cpiContext: nullable(PublicKeyFromString),
});
const TreeInfoResultV2 = pick({
    treeType: number(),
    tree: PublicKeyFromString,
    queue: PublicKeyFromString,
    cpiContext: nullable(PublicKeyFromString),
    nextTreeContext: optional(nullable(NextTreeInfoResultV2)),
});

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
            discriminator: BNFromStringOrNumber,
        }),
    ),
    lamports: BNFromStringOrNumber,
    owner: PublicKeyFromString,
    leafIndex: number(),
    tree: PublicKeyFromString,
    seq: nullable(BNFromStringOrNumber),
    slotCreated: BNFromStringOrNumber,
});

export const CompressedAccountResultV2 = pick({
    address: nullable(ArrayFromString),
    hash: BN254FromString,
    data: nullable(
        pick({
            data: Base64EncodedCompressedAccountDataResult,
            dataHash: BN254FromString,
            discriminator: BNFromStringOrNumber,
        }),
    ),
    lamports: BNFromStringOrNumber,
    owner: PublicKeyFromString,
    leafIndex: number(),
    seq: nullable(BNFromStringOrNumber),
    slotCreated: BNFromStringOrNumber,
    merkleContext: TreeInfoResultV2,
    proveByIndex: BooleanFromNumberOrBoolean,
});

export const TokenDataResult = pick({
    mint: PublicKeyFromString,
    owner: PublicKeyFromString,
    amount: BNFromStringOrNumber,
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
export const CompressedTokenAccountResultV2 = pick({
    tokenData: TokenDataResult,
    account: CompressedAccountResultV2,
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
export const MultipleCompressedAccountsResultV2 = pick({
    items: array(CompressedAccountResultV2),
});

/**
 * @internal
 */
export const CompressedAccountsByOwnerResult = pick({
    items: array(CompressedAccountResult),
    cursor: nullable(string()),
});

/**
 * @internal
 */
export const CompressedAccountsByOwnerResultV2 = pick({
    items: array(CompressedAccountResultV2),
    cursor: nullable(string()),
});
/**
 * @internal
 */
export const CompressedTokenAccountsByOwnerOrDelegateResult = pick({
    items: array(CompressedTokenAccountResult),
    cursor: nullable(string()),
});

/**
 * @internal
 */
export const CompressedTokenAccountsByOwnerOrDelegateResultV2 = pick({
    items: array(CompressedTokenAccountResultV2),
    cursor: nullable(string()),
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
export const MerkleProofResult = pick({
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
export const MerkleProofResultV2 = pick({
    hash: BN254FromString,
    leafIndex: number(),
    proof: array(BN254FromString),
    root: BN254FromString,
    rootSeq: number(),
    proveByIndex: BooleanFromNumberOrBoolean,
    treeContext: TreeInfoResultV2,
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
export const RootIndexResultV2 = pick({
    rootIndex: number(),
    proveByIndex: BooleanFromNumberOrBoolean,
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

const AccountProofInputsResult = pick({
    hash: BN254FromString,
    root: BN254FromString,
    rootIndex: RootIndexResultV2,
    merkleContext: TreeInfoResultV2,
    leafIndex: number(),
});
const AddressProofInputsResult = pick({
    address: BN254FromString,
    root: BN254FromString,
    rootIndex: number(),
    merkleContext: TreeInfoResultV2,
});

export const ValidityProofResultV2 = pick({
    compressedProof: nullable(CompressedProofResult),
    accounts: array(AccountProofInputsResult),
    addresses: array(AddressProofInputsResult),
});

/**
 * @internal
 */
export const MultipleMerkleProofsResult = array(MerkleProofResult);

/**
 * @internal
 */
export const MultipleMerkleProofsResultV2 = array(MerkleProofResultV2);

/**
 * @internal
 */
export const BalanceResult = pick({
    amount: BNFromStringOrNumber,
});

export const NativeBalanceResult = BNFromStringOrNumber;

export const TokenBalanceResult = pick({
    balance: BNFromStringOrNumber,
    mint: PublicKeyFromString,
});

export const TokenBalanceListResult = pick({
    tokenBalances: array(TokenBalanceResult),
    cursor: nullable(string()),
});

export const TokenBalanceListResultV2 = pick({
    items: array(TokenBalanceResult),
    cursor: nullable(string()),
});

export const CompressedMintTokenHoldersResult = pick({
    cursor: nullable(string()),
    items: array(
        pick({
            balance: BNFromStringOrNumber,
            owner: PublicKeyFromString,
        }),
    ),
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
    cursor: nullable(string()),
});

/**
 * @internal
 */
const ClosedAccountResultV2 = pick({
    account: CompressedAccountResultV2,
    txHash: BN254FromString,
    nullifier: BN254FromString,
});

/**
 * @internal
 */

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

/**
 * @internal
 */
export const CompressedTransactionResultV2 = pick({
    compressionInfo: pick({
        closedAccounts: array(
            pick({
                account: ClosedAccountResultV2,
                optionalTokenData: nullable(TokenDataResult),
            }),
        ),
        openedAccounts: array(
            pick({
                account: CompressedAccountResultV2,
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
    ): Promise<ValidityProofWithContext>;

    getValidityProofV0(
        hashes: HashWithTree[],
        newAddresses: AddressWithTree[],
    ): Promise<ValidityProofWithContext>;

    getValidityProofAndRpcContext(
        hashes: HashWithTree[],
        newAddresses: AddressWithTree[],
    ): Promise<WithContext<ValidityProofWithContext>>;

    getCompressedAccountsByOwner(
        owner: PublicKey,
        config?: GetCompressedAccountsByOwnerConfig,
    ): Promise<WithCursor<CompressedAccountWithMerkleContext[]>>;

    getCompressedMintTokenHolders(
        mint: PublicKey,
        options?: PaginatedOptions,
    ): Promise<WithContext<WithCursor<CompressedMintTokenHolders[]>>>;

    getCompressedTokenAccountsByOwner(
        publicKey: PublicKey,
        options: GetCompressedTokenAccountsByOwnerOrDelegateOptions,
    ): Promise<WithCursor<ParsedTokenAccount[]>>;

    getCompressedTokenAccountsByDelegate(
        delegate: PublicKey,
        options: GetCompressedTokenAccountsByOwnerOrDelegateOptions,
    ): Promise<WithCursor<ParsedTokenAccount[]>>;

    getCompressedTokenAccountBalance(hash: BN254): Promise<{ amount: BN }>;

    getCompressedTokenBalancesByOwner(
        publicKey: PublicKey,
        options: GetCompressedTokenAccountsByOwnerOrDelegateOptions,
    ): Promise<WithCursor<TokenBalance[]>>;

    getCompressedTokenBalancesByOwnerV2(
        publicKey: PublicKey,
        options: GetCompressedTokenAccountsByOwnerOrDelegateOptions,
    ): Promise<WithContext<WithCursor<TokenBalance[]>>>;

    getTransactionWithCompressionInfo(
        signature: string,
    ): Promise<CompressedTransaction | null>;

    getCompressionSignaturesForAccount(
        hash: BN254,
    ): Promise<SignatureWithMetadata[]>;

    getCompressionSignaturesForAddress(
        address: PublicKey,
        options?: PaginatedOptions,
    ): Promise<WithCursor<SignatureWithMetadata[]>>;

    getCompressionSignaturesForOwner(
        owner: PublicKey,
        options?: PaginatedOptions,
    ): Promise<WithCursor<SignatureWithMetadata[]>>;

    getCompressionSignaturesForTokenOwner(
        owner: PublicKey,
        options?: PaginatedOptions,
    ): Promise<WithCursor<SignatureWithMetadata[]>>;

    getLatestNonVotingSignatures(
        limit?: number,
        cursor?: string,
    ): Promise<LatestNonVotingSignatures>;

    getLatestCompressionSignatures(
        cursor?: string,
        limit?: number,
    ): Promise<LatestNonVotingSignaturesPaginated>;

    getIndexerHealth(): Promise<string>;

    getIndexerSlot(): Promise<number>;
}

// Public types for consumers
export type RpcResultSuccess<T> = {
    jsonrpc: '2.0';
    id: string;
    result: T;
};

export type RpcResultError = {
    jsonrpc: '2.0';
    id: string;
    error: {
        code: unknown;
        message: string;
        data?: any;
    };
};

export type RpcResult<T> = RpcResultSuccess<T> | RpcResultError;
