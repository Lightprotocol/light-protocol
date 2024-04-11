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

// TODO: consistent types
export type CompressedProofWithContext = {
    compressedProof: CompressedProof;
    roots: string[];
    // for now we assume latest root = allLeaves.length
    rootIndices: number[];
    leafIndices: number[];
    leaves: BN[];
    merkleTree: PublicKey;
    nullifierQueue: PublicKey;
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
    nullable(string()),
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
/// Compressed Account With Merkle Context
export const CompressedAccountResult = pick({
    hash: BN254FromString,
    address: nullable(PublicKeyFromString),
    data: Base64EncodedCompressedAccountDataResult,
    dataHash: nullable(BN254FromString),
    discriminator: BNFromInt,
    owner: PublicKeyFromString,
    lamports: BNFromInt,
    tree: nullable(PublicKeyFromString), // TODO: should not be nullable
    seq: nullable(BNFromInt),
    slotUpdated: BNFromInt,
    leafIndex: number(),
});

/**
 * @internal
 */
/// TODO: update: delegatedAmount, state, programOwner/tokenOwner, data includes the values?, no closeAuth!
export const CompressedTokenAccountResult = pick({
    address: nullable(PublicKeyFromString), // TODO: why is this here
    amount: BNFromBase10String, // why string
    delegate: nullable(PublicKeyFromString),
    closeAuthority: nullable(PublicKeyFromString), // TODO: remove
    isNative: boolean(),
    frozen: boolean(),
    mint: PublicKeyFromString,
    owner: PublicKeyFromString, // owner or user?
    //
    hash: BN254FromString,
    data: Base64EncodedCompressedAccountDataResult,
    dataHash: nullable(BN254FromString),
    discriminator: BNFromInt,
    lamports: BNFromInt,
    tree: PublicKeyFromString,
    seq: BNFromInt,
    // slotUpdated: BNFromInt, TODO: add owner (?): TODO: check whether this
    // implicitly assumes tokenprogram as account owner
    leafIndex: number(),
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
    // cursor: array(number()), // paginated
});

/**
 * @internal
 */
export const CompressedTokenAccountsByOwnerOrDelegateResult = pick({
    items: array(CompressedTokenAccountResult),
    // cursor: array(number()), // paginated TODO: add cursor to photon / docs update
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
    merkleTree: PublicKeyFromString,
    leafIndex: number(),
    proof: array(BN254FromString),
});

/**
 * @internal
 */
export const MultipleMerkleProofsResult = array(MerkeProofResult);

/**
 * @internal
 */
export const BalanceResult = BNFromInt;

/// TODO: we need to add: tree, nullifierQueue, leafIndex, rootIndex
export const AccountProofResult = pick({
    hash: array(number()),
    root: array(number()),
    proof: array(array(number())),
});

export interface CompressionApiInterface {
    /** Retrieve compressed account by hash or address */
    getCompressedAccount(
        hash: BN254,
    ): Promise<CompressedAccountWithMerkleContext | null>;

    /** Retrieve compressed account by hash or address */
    getCompressedBalance(hash: BN254): Promise<BN | null>;

    /** Retrieve merkle proof for a compressed account */
    getCompressedAccountProof(
        hash: BN254,
    ): Promise<MerkleContextWithMerkleProof>; // TODO: expose context slot

    /** Retrieve compressed account by hash or address */
    getMultipleCompressedAccounts(
        hashes: BN254[],
    ): Promise<CompressedAccountWithMerkleContext[] | null>;

    /** Retrieve multiple merkle proofs for compressed accounts */
    getMultipleCompressedAccountProofs(
        hashes: BN254[],
    ): Promise<MerkleContextWithMerkleProof[] | null>;

    /** Retrieve compressed accounts by owner */
    getCompressedAccountsByOwner(
        owner: PublicKey,
    ): Promise<CompressedAccountWithMerkleContext[] | null>;

    /** Receive validity Proof for n compressed accounts */
    getValidityProof(hashes: BN254[]): Promise<CompressedProofWithContext>;

    /** Retrieve health status of the node */
    getHealth(): Promise<string>;

    /** Retrieve the current slot */
    getSlot(): Promise<number>;

    getCompressedTokenAccountsByOwner(
        publicKey: PublicKey,
        options?: GetCompressedTokenAccountsByOwnerOrDelegateOptions,
    ): Promise<ParsedTokenAccount[]>;

    getCompressedTokenAccountsByDelegate(
        delegate: PublicKey,
        options?: GetCompressedTokenAccountsByOwnerOrDelegateOptions,
    ): Promise<ParsedTokenAccount[]>;

    getCompressedTokenAccountBalance(hash: BN254): Promise<{ amount: BN }>;
}
