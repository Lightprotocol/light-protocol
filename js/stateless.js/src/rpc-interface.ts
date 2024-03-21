import { PublicKey, DataSizeFilter, MemcmpFilter } from '@solana/web3.js';

import {
    type as pick,
    number,
    string,
    array,
    literal,
    union,
    optional,
    coerce,
    instance,
    create,
    tuple,
    unknown,
    any,
} from 'superstruct';
import type { Struct } from 'superstruct';
import { decodeUtxoData, isValidTlvDataElement } from './state/utxo-data';
import {
    MerkleContext,
    MerkleUpdateContext,
    UtxoWithMerkleContext,
    BN254,
    createBN254,
    TlvDataElement_IdlType,
} from './state';
import { BN } from '@coral-xyz/anchor';

export type GetCompressedAccountsFilter = MemcmpFilter | DataSizeFilter;

export type GetUtxoConfig = {
    encoding?: string;
};
export type GetCompressedAccountConfig = GetUtxoConfig;

export type GetCompressedAccountsConfig = {
    encoding?: string;
    filters?: GetCompressedAccountsFilter[];
};

export type WithMerkleUpdateContext<T> = {
    /** merkle update context */
    context: MerkleUpdateContext | null;
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
const BN254FromString = coerce(instance(BN), string(), value =>
    createBN254(value),
);

/**
 * @internal
 */
const Base64EncodedUtxoDataResult = tuple([string(), literal('base64')]);

/**
 * @internal
 */
const TlvFromBase64EncodedUtxoData = coerce(
    instance(Array<TlvDataElement_IdlType>),
    Base64EncodedUtxoDataResult,
    value => {
        const decodedData = decodeUtxoData(Buffer.from(value[0], 'base64'));
        if (decodedData.tlvElements.every(isValidTlvDataElement)) {
            return decodedData;
        } else {
            throw new Error('Invalid TlvDataElement structure');
        }
    },
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
                data: optional(any()),
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
/// Utxo with merkle context
export const UtxoResult = pick({
    data: TlvFromBase64EncodedUtxoData,
    owner: PublicKeyFromString,
    lamports: number(),
    leafIndex: number(),
    merkleTree: PublicKeyFromString,
    nullifierQueue: PublicKeyFromString,
    slotCreated: number(),
    seq: number(),
    address: optional(PublicKeyFromString),
});

/**
 * @internal
 */
/// Utxo with merkle context
export const UtxosResult = array(
    pick({
        data: TlvFromBase64EncodedUtxoData,
        hash: BN254FromString,
        lamports: number(),
        leafIndex: number(),
        merkleTree: PublicKeyFromString,
        nullifierQueue: PublicKeyFromString,
        slotCreated: number(),
        seq: number(),
        address: optional(PublicKeyFromString),
    }),
);
/**
 * @internal
 */
export const MerkleProofResult = pick({
    merkleTree: PublicKeyFromString,
    nullifierQueue: PublicKeyFromString,
    leafIndex: number(),
    proof: array(BN254FromString),
    rootIndex: number(),
});

/**
 * @internal
 */
export const CompressedAccountMerkleProofResult = pick({
    utxoHash: PublicKeyFromString,
    merkleTree: PublicKeyFromString,
    nullifierQueue: PublicKeyFromString,
    leafIndex: number(),
    proof: array(BN254FromString),
    rootIndex: number(),
});

export interface CompressionApiInterface {
    /** Retrieve a utxo */
    getUtxo(
        utxoHash: BN254,
        config?: GetUtxoConfig,
    ): Promise<WithMerkleUpdateContext<UtxoWithMerkleContext> | null>;
    /** Retrieve the proof for a utxo */
    getUtxoProof(utxoHash: BN254): Promise<MerkleContext | null>;
    /** Retrieve utxos by owner */
    getUtxos(
        owner: PublicKey,
        config?: GetUtxoConfig,
    ): Promise<WithMerkleUpdateContext<UtxoWithMerkleContext>[]>;
}
