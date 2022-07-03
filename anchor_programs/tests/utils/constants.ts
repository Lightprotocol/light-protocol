const solana = require("@solana/web3.js");
import * as anchor from "@project-serum/anchor";
import { ASSOCIATED_TOKEN_PROGRAM_ID, TOKEN_PROGRAM_ID } from '@solana/spl-token';


export const MERKLE_TREE_SIGNER_AUTHORITY = new solana.PublicKey([59, 42, 227, 2, 155, 13, 249, 77, 6, 97, 72, 159, 190, 119, 46, 110, 226, 42, 153, 232, 210, 107, 116, 255, 63, 213, 216, 18, 94, 128, 155, 225])
export const TYPE_PUBKEY = { array: [ 'u8', 32 ] };
export const TYPE_SEED = {defined: "&[u8]"};
export const TYPE_INIT_DATA = { array: [ 'u8', 642 ] };
// const constants:any = {};
// import { MerkleTreeProgram, IDL } from "../target/types/merkle_tree_program";

// IDL.constants.map((item) => {
//   if(_.isEqual(item.type, TYPE_SEED)) {
//     constants[item.name] = item.value.replace("b\"", "").replace("\"", "");
//   } else //if(_.isEqual(item.type, TYPE_PUBKEY) || _.isEqual(item.type, TYPE_INIT_DATA))
//   {
//     constants[item.name] = JSON.parse(item.value)
//   }
// });

export const PRIVATE_KEY = [
  17, 34, 231, 31, 83, 147, 93, 173, 61, 164, 25, 0, 204, 82, 234, 91, 202, 187, 228, 110, 146,
  97, 112, 131, 180, 164, 96, 220, 57, 207, 65, 107, 2, 99, 226, 251, 88, 66, 92, 33, 25, 216,
  211, 185, 112, 203, 212, 238, 105, 144, 72, 121, 176, 253, 106, 168, 115, 158, 154, 188, 62,
  255, 166, 81,
];
export const MERKLE_TREE_INIT_AUTHORITY = [2, 99, 226, 251, 88, 66, 92, 33, 25, 216, 211, 185, 112, 203, 212, 238, 105, 144, 72, 121, 176,
  253, 106, 168, 115, 158, 154, 188, 62, 255, 166, 81,
];
export const ADMIN_AUTH_KEY = new solana.PublicKey(new Uint8Array(MERKLE_TREE_INIT_AUTHORITY));
export const ADMIN_AUTH_KEYPAIR = solana.Keypair.fromSecretKey(new Uint8Array(PRIVATE_KEY));
export const MERKLE_TREE_ACC_BYTES_0 = new Uint8Array([
    242, 149, 147, 41, 62, 228, 214, 222, 231, 159, 167, 195, 10, 226, 182, 153, 84, 80, 249, 150,
    131, 112, 150, 225, 133, 131, 32, 149, 69, 188, 94, 13,
]);
export const MERKLE_TREE_KP = solana.Keypair.fromSeed(MERKLE_TREE_ACC_BYTES_0);

export const MERKLE_TREE_KEY = MERKLE_TREE_KP.publicKey;

export const MERKLE_TREE_SIZE = 16658;

export const MERKLE_TREE_TOKEN_ACC_BYTES_0 = new Uint8Array([
    123, 30, 128, 110, 93, 171, 2, 242, 20, 194, 175, 25, 246, 98, 182, 99, 31, 110, 119, 163, 68,
    179, 244, 89, 176, 19, 93, 136, 149, 231, 179, 213,
]);

export const AUTHORITY_SEED = anchor.utils.bytes.utf8.encode("AUTHORITY_SEED")
export const DEFAULT_PROGRAMS = {
  systemProgram: solana.SystemProgram.programId,
  tokenProgram: TOKEN_PROGRAM_ID,
  associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
  rent: solana.SYSVAR_RENT_PUBKEY,
  clock: solana.SYSVAR_CLOCK_PUBKEY,
};
