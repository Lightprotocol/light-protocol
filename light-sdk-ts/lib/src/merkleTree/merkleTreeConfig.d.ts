import * as anchor from "@project-serum/anchor";
import { MerkleTreeProgram } from "../../idls/merkle_tree_program";
import { Connection, PublicKey, Keypair } from "@solana/web3.js";
export declare class MerkleTreeConfig {
  merkleTreeProgram: MerkleTreeProgram;
  merkleTreePubkey: PublicKey;
  connection: Connection;
  registeredVerifierPdas: any;
  preInsertedLeavesIndex?: PublicKey;
  merkleTreeAuthorityPda?: PublicKey;
  payer: Keypair;
  tokenAuthority?: PublicKey;
  constructor({
    merkleTreePubkey,
    payer,
    connection,
  }: {
    merkleTreePubkey: PublicKey;
    payer?: Keypair;
    connection: Connection;
  });
  getPreInsertedLeavesIndex(): Promise<anchor.web3.PublicKey>;
  initializeNewMerkleTree(merkleTreePubkey?: PublicKey): Promise<any>;
  checkMerkleTreeIsInitialized(): Promise<void>;
  checkPreInsertedLeavesIndexIsInitialized(): Promise<void>;
  printMerkleTree(): Promise<void>;
  getMerkleTreeAuthorityPda(): Promise<anchor.web3.PublicKey>;
  initMerkleTreeAuthority(authority?: Keypair | undefined): Promise<any>;
  updateMerkleTreeAuthority(
    newAuthority: PublicKey,
    test?: boolean
  ): Promise<any>;
  enableNfts(configValue: Boolean): Promise<any>;
  enablePermissionlessSplTokens(configValue: Boolean): Promise<any>;
  updateLockDuration(lockDuration: Number): Promise<any>;
  getRegisteredVerifierPda(verifierPubkey: PublicKey): Promise<any>;
  registerVerifier(verifierPubkey: PublicKey): Promise<any>;
  checkVerifierIsRegistered(verifierPubkey: PublicKey): Promise<void>;
  getPoolTypePda(poolType: any): Promise<any>;
  registerPoolType(poolType: any): Promise<any>;
  checkPoolRegistered(
    poolPda: any,
    poolType: any,
    mint?: PublicKey | null
  ): Promise<void>;
  getSolPoolPda(poolType: any): Promise<any>;
  registerSolPool(poolType: any): Promise<any>;
  getSplPoolPdaToken(
    poolType: any,
    mint: PublicKey
  ): Promise<anchor.web3.PublicKey>;
  getSplPoolPda(poolType: any, mint: PublicKey): Promise<any>;
  getTokenAuthority(): Promise<anchor.web3.PublicKey>;
  registerSplPool(poolType: any, mint: PublicKey): Promise<any>;
}
