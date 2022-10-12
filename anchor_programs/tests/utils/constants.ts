const solana = require("@solana/web3.js");
import * as anchor from "@project-serum/anchor";
import { ASSOCIATED_TOKEN_PROGRAM_ID, TOKEN_PROGRAM_ID } from '@solana/spl-token';

export const FIELD_SIZE = new anchor.BN('21888242871839275222246405745257275088548364400416034343698204186575808495617');
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
export const MERKLE_TREE_HEIGHT = 18;
export const DEFAULT_ZERO = '14522046728041339886521211779101644712859239303505368468566383402165481390632';

export const PRIVATE_KEY = [
  17, 34, 231, 31, 83, 147, 93, 173, 61, 164, 25, 0, 204, 82, 234, 91, 202, 187, 228, 110, 146,
  97, 112, 131, 180, 164, 96, 220, 57, 207, 65, 107, 2, 99, 226, 251, 88, 66, 92, 33, 25, 216,
  211, 185, 112, 203, 212, 238, 105, 144, 72, 121, 176, 253, 106, 168, 115, 158, 154, 188, 62,
  255, 166, 81,
];
export const MERKLE_TREE_INIT_AUTHORITY = [2, 99, 226, 251, 88, 66, 92, 33, 25, 216, 211, 185, 112, 203, 212, 238, 105, 144, 72, 121, 176,
  253, 106, 168, 115, 158, 154, 188, 62, 255, 166, 81,
];

export const MINT_PRIVATE_KEY = new Uint8Array([
  194, 220,  38, 233, 140, 177,  44, 255, 131,   7, 129,
  209,  20, 230, 130,  41, 128, 186, 233, 161,  10,  77,
  134,  70,  34, 141,  30, 246, 145,  69,  69,  35,  14,
  129,  15,  86, 229, 176, 155,   3,   8, 217, 125,  97,
  221, 115, 252, 160, 127, 236,  37, 229, 116,  84, 111,
    6,   5, 182, 141,  86,   7,  23, 246, 215
]);

export const MINT = new solana.PublicKey([
   14, 129,  15,  86, 229, 176, 155,   3,
    8, 217, 125,  97, 221, 115, 252, 160,
  127, 236,  37, 229, 116,  84, 111,   6,
    5, 182, 141,  86,   7,  23, 246, 215
])

export const ADMIN_AUTH_KEY = new solana.PublicKey(new Uint8Array(MERKLE_TREE_INIT_AUTHORITY));
export const ADMIN_AUTH_KEYPAIR = solana.Keypair.fromSecretKey(new Uint8Array(PRIVATE_KEY));
export const MERKLE_TREE_ACC_BYTES_0 = new Uint8Array([
  190, 128,   2, 139, 132, 166, 200,
  112, 236,  75,  16,  77, 200, 175,
  154, 124, 163, 241, 240, 136,  11,
   14, 233, 211,  37, 101, 200, 190,
  101, 163, 127,  20
]);
export const MERKLE_TREE_KP = solana.Keypair.fromSeed(MERKLE_TREE_ACC_BYTES_0);

export const MERKLE_TREE_KEY = MERKLE_TREE_KP.publicKey;

export const MERKLE_TREE_SIZE = 16658;

export const MERKLE_TREE_TOKEN_ACC_BYTES_0 = new Uint8Array([
  218, 24,  22, 174,  97, 242, 114,  92,
   10, 17, 126,  18, 203, 163, 145, 123,
    3, 83, 209, 157, 145, 202, 112, 112,
  133, 88,   2, 242, 144,  12, 225,  72
]);

export const AUTHORITY_SEED = anchor.utils.bytes.utf8.encode("AUTHORITY_SEED")
export const DEFAULT_PROGRAMS = {
  systemProgram: solana.SystemProgram.programId,
  tokenProgram: TOKEN_PROGRAM_ID,
  associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
  rent: solana.SYSVAR_RENT_PUBKEY,
  clock: solana.SYSVAR_CLOCK_PUBKEY,
};
