/// <reference types="node" />
/// <reference types="bn.js" />
import { PublicKey } from "@solana/web3.js";
import * as anchor from "@coral-xyz/anchor";
import { BN, Program, Idl } from "@coral-xyz/anchor";
import { Utxo } from "../utxo";
import { Account, Relayer, Provider, Action, TokenData, transactionParameters, lightAccounts, AppUtxoConfig } from "../index";
type VerifierConfig = {
    in: number;
    out: number;
};
export declare class TransactionParameters implements transactionParameters {
    message?: Buffer;
    inputUtxos: Array<Utxo>;
    outputUtxos: Array<Utxo>;
    accounts: lightAccounts;
    relayer: Relayer;
    encryptedUtxos?: Uint8Array;
    poseidon: any;
    publicAmountSpl: BN;
    publicAmountSol: BN;
    assetPubkeys: PublicKey[];
    assetPubkeysCircuit: string[];
    action: Action;
    ataCreationFee?: boolean;
    txIntegrityHash?: BN;
    verifierIdl: Idl;
    verifierProgramId: PublicKey;
    verifierConfig: VerifierConfig;
    constructor({ message, eventMerkleTreePubkey, transactionMerkleTreePubkey, senderSpl, recipientSpl, senderSol, recipientSol, inputUtxos, outputUtxos, relayer, encryptedUtxos, poseidon, action, ataCreationFee, verifierIdl, }: {
        message?: Buffer;
        eventMerkleTreePubkey: PublicKey;
        transactionMerkleTreePubkey: PublicKey;
        senderSpl?: PublicKey;
        recipientSpl?: PublicKey;
        senderSol?: PublicKey;
        recipientSol?: PublicKey;
        inputUtxos?: Utxo[];
        outputUtxos?: Utxo[];
        relayer?: Relayer;
        encryptedUtxos?: Uint8Array;
        poseidon: any;
        action: Action;
        provider?: Provider;
        ataCreationFee?: boolean;
        verifierIdl: Idl;
    });
    toBytes(): Promise<Buffer>;
    static findIdlIndex(programId: string, idlObjects: anchor.Idl[]): number;
    static getVerifierProgramId(verifierIdl: Idl): PublicKey;
    static getVerifierProgram(verifierIdl: Idl, anchorProvider: anchor.AnchorProvider): Program<Idl>;
    static getVerifierConfig(verifierIdl: Idl): VerifierConfig;
    static fromBytes({ poseidon, utxoIdls, bytes, relayer, verifierIdl, assetLookupTable, verifierProgramLookupTable, }: {
        poseidon: any;
        utxoIdls?: anchor.Idl[];
        bytes: Buffer;
        relayer: Relayer;
        verifierIdl: Idl;
        assetLookupTable: string[];
        verifierProgramLookupTable: string[];
    }): Promise<TransactionParameters>;
    static getTxParams({ tokenCtx, publicAmountSpl, publicAmountSol, action, userSplAccount, account, utxos, inUtxos, recipientSol, recipientSplAddress, outUtxos, relayer, provider, ataCreationFee, // associatedTokenAccount = ata
    appUtxo, addInUtxos, addOutUtxos, verifierIdl, mergeUtxos, message, assetLookupTable, verifierProgramLookupTable, separateSolUtxo, }: {
        tokenCtx: TokenData;
        publicAmountSpl?: BN;
        publicAmountSol?: BN;
        userSplAccount?: PublicKey;
        account: Account;
        utxos?: Utxo[];
        recipientSol?: PublicKey;
        recipientSplAddress?: PublicKey;
        inUtxos?: Utxo[];
        outUtxos?: Utxo[];
        action: Action;
        provider: Provider;
        relayer?: Relayer;
        ataCreationFee?: boolean;
        appUtxo?: AppUtxoConfig;
        addInUtxos?: boolean;
        addOutUtxos?: boolean;
        verifierIdl: Idl;
        mergeUtxos?: boolean;
        message?: Buffer;
        assetLookupTable: string[];
        verifierProgramLookupTable: string[];
        separateSolUtxo?: boolean;
    }): Promise<TransactionParameters>;
    /**
     * @description Adds empty utxos until the desired number of utxos is reached.
     * @note The zero knowledge proof circuit needs all inputs to be defined.
     * @note Therefore, we have to pass in empty inputs for values we don't use.
     * @param utxos
     * @param len
     * @returns
     */
    addEmptyUtxos(utxos: Utxo[], len: number): Utxo[];
    /**
     * @description Assigns spl and sol senderSpl or recipientSpl accounts to transaction parameters based on action.
     */
    assignAccounts(): void;
    static getEscrowPda(verifierProgramId: PublicKey): PublicKey;
    static getAssetPubkeys(inputUtxos?: Utxo[], outputUtxos?: Utxo[]): {
        assetPubkeysCircuit: string[];
        assetPubkeys: PublicKey[];
    };
    /**
     * @description Calculates the external amount for one asset.
     * @note This function might be too specific since the circuit allows assets to be in any index
     * @param assetIndex the index of the asset the external amount should be computed for
     * @returns {BN} the public amount of the asset
     */
    static getExternalAmount(assetIndex: number, inputUtxos: Utxo[], outputUtxos: Utxo[], assetPubkeysCircuit: string[]): BN;
    /**
     * Computes the integrity Poseidon hash over transaction inputs that are not part of
     * the proof, but are included to prevent the relayer from changing any input of the
     * transaction.
     *
     * The hash is computed over the following inputs in the given order:
     * 1. Recipient SPL Account
     * 2. Recipient Solana Account
     * 3. Relayer Public Key
     * 4. Relayer Fee
     * 5. Encrypted UTXOs (limited to 512 bytes)
     *
     * @param {any} poseidon - Poseidon hash function instance.
     * @returns {Promise<BN>} A promise that resolves to the computed transaction integrity hash.
     * @throws {TransactionError} Throws an error if the relayer, recipient SPL or Solana accounts,
     * relayer fee, or encrypted UTXOs are undefined, or if the encryption of UTXOs fails.
     *
     * @example
     * const integrityHash = await getTxIntegrityHash(poseidonInstance);
     */
    getTxIntegrityHash(poseidon: any): Promise<BN>;
    encryptOutUtxos(poseidon: any, encryptedUtxos?: Uint8Array): Promise<Uint8Array>;
}
export {};
//# sourceMappingURL=transactionParameters.d.ts.map