/// <reference types="bn.js" />
export declare const createEncryptionKeypair: () => any;
import { MerkleTreeProgramIdl } from "./idls/merkle_tree_program";
import { PublicKey, Keypair } from "@solana/web3.js";
import { Utxo } from "./utxo";
import { AnchorProvider, BN, Program } from "@coral-xyz/anchor";
import { PublicInputs } from "./verifiers";
export declare class Transaction {
    relayerPubkey: PublicKey;
    relayerRecipient: PublicKey;
    preInsertedLeavesIndex: PublicKey;
    merkleTreeProgram: Program<MerkleTreeProgramIdl>;
    verifier: any;
    lookupTable: PublicKey;
    feeAsset: PublicKey;
    merkleTreePubkey: PublicKey;
    merkleTreeAssetPubkey?: PublicKey;
    merkleTree: any;
    utxos: any;
    payer: Keypair;
    provider: AnchorProvider;
    merkleTreeFeeAssetPubkey: PublicKey;
    poseidon: any;
    sendTransaction: Function;
    shuffle: Boolean;
    publicInputs: PublicInputs;
    encryptionKeypair: any;
    rootIndex: any;
    inputUtxos?: Utxo[];
    outputUtxos?: Utxo[];
    feeAmount?: BN;
    assetPubkeys?: BN[];
    inIndices?: Number[][][];
    outIndices?: Number[][][];
    relayerFee?: BN | null;
    sender?: PublicKey;
    senderFee?: PublicKey;
    recipient?: PublicKey;
    recipientFee?: PublicKey;
    mintPubkey?: PublicKey;
    externalAmountBigNumber?: BN;
    escrow?: PublicKey;
    leavesPdaPubkeys: any;
    nullifierPdaPubkeys: any;
    signerAuthorityPubkey: any;
    tokenAuthority: any;
    verifierStatePubkey: any;
    publicInputsBytes?: Number[][];
    encryptedUtxos?: Uint8Array;
    proofBytes: any;
    config?: {
        in: number;
        out: number;
    };
    /**
       * Initialize transaction
       *
       * @param encryptionKeypair encryptionKeypair used for encryption
       * @param relayerFee recipient of the unshielding
       * @param merkleTreePubkey
       * @param merkleTree
       * @param merkleTreeAssetPubkey
       * @param recipient utxos to pay with
       * @param lookupTable fee for the relayer
       * @param payer RPC connection
       * @param provider shieldedKeypair
       * @param relayerRecipient shieldedKeypair
       * @param poseidon shieldedKeypair
       * @param verifier shieldedKeypair
       * @param shuffleEnabled
    */
    constructor({ payer, //: Keypair
    encryptionKeypair, merkleTree, relayerPubkey, //PublicKey
    relayerRecipient, provider, lookupTable, //PublicKey
    poseidon, verifier, shuffleEnabled, }: {
        payer: any;
        encryptionKeypair?: any;
        merkleTree: any;
        relayerPubkey: any;
        relayerRecipient: any;
        provider: any;
        lookupTable: any;
        poseidon: any;
        verifier: any;
        shuffleEnabled?: boolean | undefined;
    });
    getRootIndex(): Promise<void>;
    prepareUtxos(): void;
    prepareTransaction(encrypedUtxos?: Uint8Array): void;
    prepareTransactionFull({ inputUtxos, outputUtxos, action, assetPubkeys, recipient, relayerFee, // public amount of the fee utxo adjustable if you want to deposit a fee utxo alongside your spl deposit
    shuffle, recipientFee, sender, merkleTreeAssetPubkey, config, encrypedUtxos }: {
        inputUtxos: Array<Utxo>;
        outputUtxos: Array<Utxo>;
        action: String;
        assetPubkeys: Array<PublicKey>;
        recipient: PublicKey;
        relayerFee: BN | null;
        shuffle: Boolean;
        recipientFee: PublicKey;
        sender: PublicKey;
        merkleTreeAssetPubkey: PublicKey;
        config: {
            in: number;
            out: number;
        };
        encrypedUtxos?: Uint8Array;
    }): Promise<void>;
    overWriteEncryptedUtxos(bytes: Uint8Array, offSet?: number): void;
    getPublicInputs(): void;
    getProof(): Promise<void>;
    checkProof(): Promise<void>;
    getPdaAddresses(): Promise<void>;
    checkBalances(): Promise<void>;
}
export declare const parseProofToBytesArray: (data: any) => Promise<any[]>;
