/// <reference types="bn.js" />
import * as anchor from "@coral-xyz/anchor";
import { MerkleTreeProgram } from "../idls/index";
import { Connection, PublicKey, Keypair } from "@solana/web3.js";
import { Program } from "@coral-xyz/anchor";
export declare class MerkleTreeConfig {
    merkleTreeProgram: Program<MerkleTreeProgram>;
    transactionMerkleTreePda?: PublicKey;
    connection: Connection;
    registeredVerifierPdas: any;
    preInsertedLeavesIndex?: PublicKey;
    merkleTreeAuthorityPda?: PublicKey;
    payer?: Keypair;
    tokenAuthority?: PublicKey;
    poolTypes: {
        tokenPdas: {
            mint: PublicKey;
            pubkey: PublicKey;
        }[];
        poolPda: PublicKey;
        poolType: Array<number>;
    }[];
    poolPdas: any;
    constructor({ payer, connection, }: {
        payer?: Keypair;
        connection: Connection;
    });
    initializeNewTransactionMerkleTree(oldTransactionMerkleTree: PublicKey, newTransactionMerkleTree: PublicKey): Promise<string>;
    checkTransactionMerkleTreeIsInitialized(transactionMerkleTreePda: PublicKey): Promise<void>;
    initializeNewEventMerkleTree(): Promise<string>;
    checkEventMerkleTreeIsInitialized(): Promise<void>;
    printMerkleTree(): Promise<void>;
    getMerkleTreeAuthorityPda(): anchor.web3.PublicKey;
    getMerkleTreeAuthorityAccountInfo(): Promise<{
        pubkey: anchor.web3.PublicKey;
        transactionMerkleTreeIndex: anchor.BN;
        eventMerkleTreeIndex: anchor.BN;
        registeredAssetIndex: anchor.BN;
        enableNfts: boolean;
        enablePermissionlessSplTokens: boolean;
        enablePermissionlessMerkleTreeRegistration: boolean;
    }>;
    getTransactionMerkleTreeIndex(): Promise<anchor.BN>;
    static getTransactionMerkleTreePda(transactionMerkleTreeIndex?: anchor.BN): anchor.web3.PublicKey;
    static getEventMerkleTreePda(eventMerkleTreeIndex?: anchor.BN): anchor.web3.PublicKey;
    initMerkleTreeAuthority(authority?: Keypair | undefined, transactionMerkleTree?: PublicKey): Promise<string>;
    isMerkleTreeAuthorityInitialized(): Promise<boolean>;
    updateMerkleTreeAuthority(newAuthority: PublicKey, test?: boolean): Promise<string>;
    enablePermissionlessSplTokens(configValue: boolean): Promise<string>;
    updateLockDuration(lockDuration: Number): Promise<string>;
    getRegisteredVerifierPda(verifierPubkey: PublicKey): Promise<any>;
    registerVerifier(verifierPubkey: PublicKey): Promise<string>;
    checkVerifierIsRegistered(verifierPubkey: PublicKey): Promise<void>;
    getPoolTypePda(poolType: Array<number>): Promise<{
        tokenPdas: {
            mint: anchor.web3.PublicKey;
            pubkey: anchor.web3.PublicKey;
        }[];
        poolPda: anchor.web3.PublicKey;
        poolType: number[];
    }>;
    registerPoolType(poolType: Array<number>): Promise<string>;
    checkPoolRegistered(poolPda: any, poolType: Array<number>, mint?: PublicKey | null): Promise<void>;
    static getSolPoolPda(programId: PublicKey, poolType?: Array<number>): {
        pda: anchor.web3.PublicKey;
        poolType: number[];
    };
    registerSolPool(poolType: Array<number>): Promise<string>;
    static getSplPoolPdaToken(mint: PublicKey, programId: PublicKey, poolType?: Array<number>): anchor.web3.PublicKey;
    getSplPoolPda(mint: PublicKey, poolType?: Array<number>): Promise<any>;
    getTokenAuthority(): Promise<anchor.web3.PublicKey>;
    registerSplPool(poolType: Array<number>, mint: PublicKey): Promise<string>;
}
