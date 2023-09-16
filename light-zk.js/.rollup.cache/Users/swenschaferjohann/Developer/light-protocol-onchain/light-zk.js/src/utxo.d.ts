/// <reference types="bn.js" />
/// <reference types="node" />
import { PublicKey } from "@solana/web3.js";
import { BN, Idl } from "@coral-xyz/anchor";
import { Account } from "./index";
export declare const newNonce: () => Uint8Array;
export declare const N_ASSETS = 2;
export declare const N_ASSET_PUBKEYS = 3;
export declare class Utxo {
    /**
     * @param {BN[]} amounts array of utxo amounts, amounts[0] is the sol amount amounts[1] is the spl amount
     * @param {PublicKey[]} assets  array of utxo assets, assets[0] is the sol asset assets[1] is the spl asset
     * @param {BN} blinding Blinding factor, a 31 bytes big number, to add randomness to the commitment hash.
     * @param {Account} account the account owning the utxo.
     * @param {index} index? the index of the utxo's commitment hash in the Merkle tree.
     * @param {Array<any>} appData application data of app utxos not provided for normal utxos.
     * @param {PublicKey} verifierAddress the solana address of the verifier, SystemProgramId/BN(0) for system verifiers.
     * @param {BN} verifierAddressCircuit hashAndTruncateToCircuit(verifierAddress) to fit into 254 bit field size of bn254.
     * @param {BN} appDataHash is the poseidon hash of app utxo data. This compresses the app data and ties it to the app utxo.
     * @param {BN} poolType is the pool type domain of the utxo default is [0;32].
     * @param {boolean} includeAppData flag whether to include app data when serializing utxo to bytes.
     * @param {string} _commitment cached commitment hash to avoid recomputing.
     * @param {string} _nullifier cached nullifier hash to avoid recomputing.
     */
    amounts: BN[];
    assets: PublicKey[];
    assetsCircuit: BN[];
    blinding: BN;
    account: Account;
    index?: number;
    appData: any;
    verifierAddress: PublicKey;
    verifierAddressCircuit: BN;
    appDataHash: BN;
    poolType: BN;
    _commitment?: string;
    _nullifier?: string;
    includeAppData: boolean;
    transactionVersion: string;
    appDataIdl?: Idl;
    splAssetIndex: BN;
    verifierProgramIndex: BN;
    /**
     * @description Initialize a new utxo - unspent transaction output or input. Note, a full TX consists of 2 inputs and 2 outputs
     *
     * @param {BN[]} amounts array of utxo amounts, amounts[0] is the sol amount amounts[1] is the spl amount
     * @param {PublicKey[]} assets  array of utxo assets, assets[0] is the sol asset assets[1] is the spl asset
     * @param {BN} blinding Blinding factor, a 31 bytes big number, to add randomness to the commitment hash.
     * @param {Account} account the account owning the utxo.
     * @param {index} index? the index of the utxo's commitment hash in the Merkle tree.
     * @param {Array<any>} appData application data of app utxos not provided for normal utxos.
     * @param {PublicKey} verifierAddress the solana address of the verifier, SystemProgramId/BN(0) for system verifiers.
     * @param {BN} appDataHash is the poseidon hash of app utxo data. This compresses the app data and ties it to the app utxo.
     * @param {any} poseidon poseidon hasher instance.
     * @param {boolean} includeAppData flag whether to include app data when serializing utxo to bytes.
     * @param {function} appDataFromBytesFn function to deserialize appData from bytes.
     * @param {appData} appData array of application data, is used to compute the instructionDataHash.
     */
    constructor({ poseidon, assets, amounts, account, blinding, poolType, verifierAddress, index, appDataHash, appData, appDataIdl, includeAppData, assetLookupTable, verifierProgramLookupTable, }: {
        poseidon: any;
        assets?: PublicKey[];
        amounts?: BN[];
        account?: Account;
        blinding?: BN;
        poolType?: BN;
        verifierAddress?: PublicKey;
        index?: number;
        appData?: any;
        appDataIdl?: Idl;
        includeAppData?: boolean;
        appDataHash?: BN;
        assetLookupTable: string[];
        verifierProgramLookupTable: string[];
    });
    /**
     * @description Parses a utxo to bytes.
     * @returns {Uint8Array}
     */
    toBytes(compressed?: boolean): Promise<Buffer>;
    /**
     * @description Parses a utxo from bytes.
     * @param poseidon poseidon hasher instance
     * @param bytes byte array of a serialized utxo
     * @param account account of the utxo
     * @param appDataFromBytesFn function to parse app data from bytes
     * @param includeAppData whether to include app data when encrypting or not
     * @returns {Utxo}
     */
    static fromBytes({ poseidon, bytes, account, includeAppData, index, appDataIdl, verifierAddress, assetLookupTable, verifierProgramLookupTable, }: {
        poseidon: any;
        bytes: Buffer;
        account?: Account;
        includeAppData?: boolean;
        index?: number;
        appDataIdl?: Idl;
        verifierAddress?: PublicKey;
        assetLookupTable: string[];
        verifierProgramLookupTable: string[];
    }): Utxo;
    /**
     * @description Returns commitment for this utxo
     * @description PoseidonHash(amountHash, shieldedPubkey, blinding, assetHash, appDataHash, poolType, verifierAddressCircuit)
     * @returns {string}
     */
    getCommitment(poseidon: any): string;
    /**
     * @description Computes the nullifier for this utxo.
     * @description PoseidonHash(commitment, index, signature)
     * @param {number} index Merkle tree index of the utxo commitment (Optional)
     *
     * @returns {string}
     */
    getNullifier(poseidon: any, index?: number | undefined): string | undefined;
    /**
     * @description Encrypts the utxo to the utxo's accounts public key with nacl.box.
     *
     * @returns {Uint8Array} with the first 24 bytes being the nonce
     */
    encrypt(poseidon: any, merkleTreePdaPublicKey?: PublicKey, compressed?: boolean): Promise<Uint8Array>;
    /**
     * @description Decrypts a utxo from an array of bytes, the last 24 bytes are the nonce.
     * @param {any} poseidon
     * @param {Uint8Array} encBytes
     * @param {Account} account
     * @param {number} index
     * @returns {Utxo | null}
     */
    static decrypt({ poseidon, encBytes, account, index, merkleTreePdaPublicKey, aes, commitment, appDataIdl, compressed, assetLookupTable, verifierProgramLookupTable, }: {
        poseidon: any;
        encBytes: Uint8Array;
        account: Account;
        index: number;
        merkleTreePdaPublicKey?: PublicKey;
        aes?: boolean;
        commitment: Uint8Array;
        appDataIdl?: Idl;
        compressed?: boolean;
        assetLookupTable: string[];
        verifierProgramLookupTable: string[];
    }): Promise<Utxo | null>;
    /**
     * Creates a new Utxo from a given base58 encoded string.
     * @static
     * @param {string} string - The base58 encoded string representation of the Utxo.
     * @returns {Utxo} The newly created Utxo.
     */
    static fromString(string: string, poseidon: any, assetLookupTable: string[], verifierProgramLookupTable: string[]): Utxo;
    /**
     * Converts the Utxo instance into a base58 encoded string.
     * @async
     * @returns {Promise<string>} A promise that resolves to the base58 encoded string representation of the Utxo.
     */
    toString(): Promise<string>;
    /**
     * @description Compares two Utxos.
     * @param {Utxo} utxo0
     * @param {Utxo} utxo1
     */
    static equal(poseidon: any, utxo0: Utxo, utxo1: Utxo, skipNullifier?: boolean): void;
    static getAppInUtxoIndices(appUtxos: Utxo[]): any[][];
}
//# sourceMappingURL=utxo.d.ts.map