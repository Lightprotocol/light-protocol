/// <reference types="bn.js" />
import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { VerifierProgramTwo, VerifierProgramOne, VerifierProgramZero, MerkleTreeProgram } from "./idls/index";
import { ConfirmOptions } from "@solana/web3.js";
import { TokenData } from "./index";
export declare const BN_0: anchor.BN;
export declare const BN_1: anchor.BN;
export declare const BN_2: anchor.BN;
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
export declare const verifierProgramStorageProgramId: anchor.web3.PublicKey;
export declare const verifierProgramZeroProgramId: anchor.web3.PublicKey;
export declare const verifierProgramOneProgramId: anchor.web3.PublicKey;
export declare const verifierProgramTwoProgramId: anchor.web3.PublicKey;
export declare const LOOK_UP_TABLE: anchor.web3.PublicKey;
export declare const MAX_U64: anchor.BN;
export declare const VERIFIER_PUBLIC_KEYS: anchor.web3.PublicKey[];
export type merkleTreeProgram = Program<MerkleTreeProgram>;
export type verifierProgramZero = Program<VerifierProgramZero>;
export type verifierProgramOne = Program<VerifierProgramOne>;
export type verifierProgramTwo = Program<VerifierProgramTwo>;
export declare const confirmConfig: ConfirmOptions;
export declare const COMPRESSED_UTXO_BYTES_LENGTH: number;
export declare const ENCRYPTED_COMPRESSED_UTXO_BYTES_LENGTH: number;
export declare const NACL_ENCRYPTED_COMPRESSED_UTXO_BYTES_LENGTH: number;
export declare const UNCOMPRESSED_UTXO_BYTES_LENGTH: number;
export declare const ENCRYPTED_UNCOMPRESSED_UTXO_BYTES_LENGTH: number;
export declare const DEFAULT_PRIVATE_KEY: string;
export declare const DEFAULT_ZERO = "14522046728041339886521211779101644712859239303505368468566383402165481390632";
export declare const AUTHORITY_SEED: Uint8Array;
export declare const DEFAULT_PROGRAMS: {
    systemProgram: anchor.web3.PublicKey;
    tokenProgram: anchor.web3.PublicKey;
    associatedTokenProgram: anchor.web3.PublicKey;
    rent: anchor.web3.PublicKey;
    clock: anchor.web3.PublicKey;
};
export declare const MINIMUM_LAMPORTS: anchor.BN;
export declare const TOKEN_ACCOUNT_FEE: anchor.BN;
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
export declare const POOL_TYPE: any[];
export declare const MERKLE_TREE_AUTHORITY_PDA: anchor.web3.PublicKey;
export declare const TESTNET_LOOK_UP_TABLE: anchor.web3.PublicKey;
export declare const FEE_ASSET: anchor.web3.PublicKey;
export declare const MERKLE_TREE_HEIGHT = 18;
/** Threshold (per asset) at which new in-UTXOs get merged, in order to reduce UTXO pool size */
export declare const UTXO_MERGE_THRESHOLD = 20;
export declare const UTXO_MERGE_MAXIMUM = 10;
export declare const UTXO_FEE_ASSET_MINIMUM = 100000;
export declare const SIGN_MESSAGE: string;
export declare const RELAYER_FEE: anchor.BN;
export declare const MAX_MESSAGE_SIZE = 800;
export declare const SOL_DECIMALS: anchor.BN;
export declare const TOKEN_REGISTRY: Map<string, TokenData>;
export declare const TOKEN_PUBKEY_SYMBOL: Map<string, string>;
/**
 * Treshold after which the currently used transaction Merkle tree is switched
 * to the next one. The limit of each merkle tree is 256k, but we want to have
 * a margin.
 */
export declare const TRANSACTION_MERKLE_TREE_SWITCH_TRESHOLD: anchor.BN;
//# sourceMappingURL=constants.d.ts.map