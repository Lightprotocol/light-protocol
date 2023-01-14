/// <reference types="bn.js" />
export declare const createEncryptionKeypair: () => any;
import { PublicKey, Keypair as SolanaKeypair, TransactionSignature, TransactionInstruction } from "@solana/web3.js";
import { BN, Program, Provider } from "@coral-xyz/anchor";
import { Utxo } from "./utxo";
import { PublicInputs, Verifier } from "./verifiers";
import { Relayer, SolMerkleTree } from "./index";
import { MerkleTreeProgramIdl } from "./idls/merkle_tree_program";
export type transactionParameters = {
    inputUtxos?: Array<Utxo>;
    outputUtxos?: Array<Utxo>;
    accounts: {
        sender?: PublicKey;
        recipient?: PublicKey;
        senderFee?: PublicKey;
        recipientFee?: PublicKey;
        verifierState?: PublicKey;
        tokenAuthority?: PublicKey;
        escrow?: PublicKey;
    };
    encryptedUtxos?: Uint8Array;
    verifier: Verifier;
    nullifierPdaPubkeys?: {
        isSigner: boolean;
        isWritable: boolean;
        pubkey: PublicKey;
    }[];
    leavesPdaPubkeys?: {
        isSigner: boolean;
        isWritable: boolean;
        pubkey: PublicKey;
    }[];
};
export declare class TransactionParameters implements transactionParameters {
    inputUtxos?: Array<Utxo>;
    outputUtxos?: Array<Utxo>;
    accounts: {
        sender?: PublicKey;
        recipient?: PublicKey;
        senderFee?: PublicKey;
        recipientFee?: PublicKey;
        verifierState?: PublicKey;
        tokenAuthority?: PublicKey;
        escrow?: PublicKey;
        systemProgramId: PublicKey;
        merkleTree: PublicKey;
        tokenProgram: PublicKey;
        registeredVerifierPda: PublicKey;
        authority: PublicKey;
        signingAddress?: PublicKey;
        preInsertedLeavesIndex: PublicKey;
        programMerkleTree: PublicKey;
    };
    encryptedUtxos?: Uint8Array;
    verifier: Verifier;
    nullifierPdaPubkeys?: {
        isSigner: boolean;
        isWritable: boolean;
        pubkey: PublicKey;
    }[];
    leavesPdaPubkeys?: {
        isSigner: boolean;
        isWritable: boolean;
        pubkey: PublicKey;
    }[];
    merkleTreeProgram?: Program<MerkleTreeProgramIdl>;
    constructor({ merkleTreePubkey, verifier, sender, recipient, senderFee, recipientFee, inputUtxos, outputUtxos, }: {
        merkleTreePubkey: PublicKey;
        verifier: Verifier;
        sender?: PublicKey;
        recipient?: PublicKey;
        senderFee?: PublicKey;
        recipientFee?: PublicKey;
        inputUtxos?: Utxo[];
        outputUtxos?: Utxo[];
    });
}
export type LightInstance = {
    provider?: Provider;
    lookUpTable?: PublicKey;
    solMerkleTree?: SolMerkleTree;
};
export declare class Transaction {
    merkleTreeProgram?: Program<MerkleTreeProgramIdl>;
    payer?: SolanaKeypair;
    poseidon: any;
    shuffleEnabled: Boolean;
    action?: string;
    params?: TransactionParameters;
    relayer: Relayer;
    instance: LightInstance;
    publicInputs?: PublicInputs;
    rootIndex: any;
    proofBytes: any;
    encryptedUtxos?: Uint8Array;
    proofInput: any;
    assetPubkeysCircuit?: BN[];
    assetPubkeys?: PublicKey[];
    publicAmount?: BN;
    feeAmount?: BN;
    inputMerklePathIndices?: number[];
    inputMerklePathElements?: number[];
    publicInputsBytes?: number[][];
    recipientBalancePriorTx?: BN;
    relayerRecipientAccountBalancePriorLastTx?: BN;
    /**
     * Initialize transaction
     *
     * @param instance encryptionKeypair used for encryption
     * @param relayer recipient of the unshielding
     * @param payer
     * @param shuffleEnabled
     */
    constructor({ instance, relayer, payer, shuffleEnabled, }: {
        instance: LightInstance;
        relayer?: Relayer;
        payer?: SolanaKeypair;
        shuffleEnabled?: boolean;
    });
    proveAndCreateInstructionsJson(params: TransactionParameters): Promise<string[]>;
    proveAndCreateInstructions(params: TransactionParameters): Promise<TransactionInstruction[]>;
    compileAndProve(params: TransactionParameters): Promise<void>;
    compile(params: TransactionParameters): Promise<void>;
    getProofInput(): void;
    getProof(): Promise<void>;
    getConnectingHash(): string;
    assignAccounts(params: TransactionParameters): void;
    getAssetPubkeys(inputUtxos?: Utxo[], outputUtxos?: Utxo[]): {
        assetPubkeysCircuit: BN[];
        assetPubkeys: PublicKey[];
    };
    getRootIndex(): Promise<void>;
    addEmptyUtxos(utxos: Utxo[] | undefined, len: number): Utxo[];
    getExternalAmount(assetIndex: number): BN;
    getIndices(utxos: Utxo[]): string[][][];
    getMerkleProofs(): void;
    getTxIntegrityHash(): BN;
    encryptOutUtxos(encryptedUtxos?: Uint8Array): Uint8Array | undefined;
    overWriteEncryptedUtxos(bytes: Uint8Array, offSet?: number): void;
    getPublicInputs(): void;
    getTestValues(): Promise<void>;
    static getSignerAuthorityPda(merkleTreeProgramId: PublicKey, verifierProgramId: PublicKey): PublicKey;
    static getRegisteredVerifierPda(merkleTreeProgramId: PublicKey, verifierProgramId: PublicKey): PublicKey;
    getInstructionsJson(): Promise<string[]>;
    sendTransaction(ix: any): Promise<TransactionSignature | undefined>;
    sendAndConfirmTransaction(): Promise<TransactionSignature>;
    checkProof(): Promise<void>;
    getPdaAddresses(): Promise<void>;
    checkBalances(): Promise<void>;
    shuffleUtxos(utxos: Utxo[]): Utxo[] | undefined;
    static parseProofToBytesArray(data: any): Promise<any[]>;
}
