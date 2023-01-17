import * as anchor from "@coral-xyz/anchor";
import { MerkleTreeProgram } from "../idls/merkle_tree_program";
import { Connection, PublicKey, Keypair } from "@solana/web3.js";
import { Program } from "@coral-xyz/anchor";
export declare class MerkleTreeConfig {
    merkleTreeProgram: Program<MerkleTreeProgram>;
    merkleTreePubkey: PublicKey;
    connection: Connection;
    registeredVerifierPdas: any;
    preInsertedLeavesIndex?: PublicKey;
    merkleTreeAuthorityPda?: PublicKey;
    payer: Keypair;
    tokenAuthority?: PublicKey;
    constructor({ merkleTreePubkey, payer, connection, }: {
        merkleTreePubkey: PublicKey;
        payer?: Keypair;
        connection: Connection;
    });
    getPreInsertedLeavesIndex(): Promise<anchor.web3.PublicKey>;
    initializeNewMerkleTree(merkleTreePubkey?: PublicKey): Promise<string>;
    checkMerkleTreeIsInitialized(): Promise<void>;
    checkPreInsertedLeavesIndexIsInitialized(): Promise<void>;
    printMerkleTree(): Promise<void>;
    getMerkleTreeAuthorityPda(): Promise<anchor.web3.PublicKey>;
    initMerkleTreeAuthority(authority?: Keypair | undefined): Promise<string>;
    updateMerkleTreeAuthority(newAuthority: PublicKey, test?: boolean): Promise<string>;
    enableNfts(configValue: Boolean): Promise<string>;
    enablePermissionlessSplTokens(configValue: Boolean): Promise<string>;
    updateLockDuration(lockDuration: Number): Promise<string>;
    getRegisteredVerifierPda(verifierPubkey: PublicKey): Promise<any>;
    registerVerifier(verifierPubkey: PublicKey): Promise<string>;
    checkVerifierIsRegistered(verifierPubkey: PublicKey): Promise<void>;
    getPoolTypePda(poolType: any): Promise<any>;
    registerPoolType(poolType: any): Promise<string>;
    checkPoolRegistered(poolPda: any, poolType: any, mint?: PublicKey | null): Promise<void>;
    static getSolPoolPda(programId: PublicKey, poolType?: Uint8Array): {
        pda: anchor.web3.PublicKey;
        poolType: Uint8Array;
    };
    registerSolPool(poolType: Uint8Array): Promise<string>;
    static getSplPoolPdaToken(mint: PublicKey, programId: PublicKey, poolType?: Uint8Array): anchor.web3.PublicKey;
    getSplPoolPda(mint: PublicKey, poolType?: Uint8Array): Promise<any>;
    getTokenAuthority(): Promise<anchor.web3.PublicKey>;
    registerSplPool(poolType: Uint8Array, mint: PublicKey): Promise<string>;
}
