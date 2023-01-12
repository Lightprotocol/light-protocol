/// <reference types="bn.js" />
import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { VerifierProgramTwoIdl, VerifierProgramOneIdl, VerifierProgramZeroIdl, MerkleTreeProgramIdl } from "./idls/index";
import { ConfirmOptions } from "@solana/web3.js";
export declare const CONSTANT_SECRET_AUTHKEY: Uint8Array;
export declare const FIELD_SIZE: anchor.BN;
export declare const MERKLE_TREE_SIGNER_AUTHORITY: anchor.web3.PublicKey;
export declare const TYPE_PUBKEY: {
    array: (string | number)[];
};
export declare const TYPE_SEED: {
    defined: string;
};
export declare const TYPE_INIT_DATA: {
    array: (string | number)[];
};
export declare const merkleTreeProgramId: anchor.web3.PublicKey;
export declare const verifierProgramZeroProgramId: anchor.web3.PublicKey;
export declare const verifierProgramOneProgramId: anchor.web3.PublicKey;
export declare const verifierProgramTwoProgramId: anchor.web3.PublicKey;
export type merkleTreeProgram = Program<MerkleTreeProgramIdl>;
export type verifierProgramZero = Program<VerifierProgramZeroIdl>;
export type verifierProgramOne = Program<VerifierProgramOneIdl>;
export type verifierProgramTwo = Program<VerifierProgramTwoIdl>;
export declare const confirmConfig: ConfirmOptions;
export declare const DEFAULT_ZERO = "14522046728041339886521211779101644712859239303505368468566383402165481390632";
export declare const AUTHORITY_SEED: Uint8Array;
export declare const DEFAULT_PROGRAMS: {
    systemProgram: anchor.web3.PublicKey;
    tokenProgram: anchor.web3.PublicKey;
    associatedTokenProgram: anchor.web3.PublicKey;
    rent: anchor.web3.PublicKey;
    clock: anchor.web3.PublicKey;
};
export declare const MERKLE_TREE_KEY: anchor.web3.PublicKey;
export declare const REGISTERED_VERIFIER_PDA: anchor.web3.PublicKey;
export declare const REGISTERED_VERIFIER_ONE_PDA: anchor.web3.PublicKey;
export declare const REGISTERED_VERIFIER_TWO_PDA: anchor.web3.PublicKey;
export declare const AUTHORITY: anchor.web3.PublicKey;
export declare const AUTHORITY_ONE: anchor.web3.PublicKey;
export declare const PRE_INSERTED_LEAVES_INDEX: anchor.web3.PublicKey;
export declare const TOKEN_AUTHORITY: anchor.web3.PublicKey;
export declare const REGISTERED_POOL_PDA_SPL: anchor.web3.PublicKey;
export declare const REGISTERED_POOL_PDA_SPL_TOKEN: anchor.web3.PublicKey;
export declare const REGISTERED_POOL_PDA_SOL: anchor.web3.PublicKey;
export declare const POOL_TYPE: Uint8Array;
export declare const MERKLE_TREE_AUTHORITY_PDA: anchor.web3.PublicKey;
export declare const FEE_ASSET: anchor.web3.PublicKey;
