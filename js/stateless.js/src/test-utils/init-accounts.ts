// TODO: remove!
// import {
//   Connection,
//   PublicKey,
//   Transaction,
//   SystemProgram,
//   LAMPORTS_PER_SOL,
//   Keypair,
// } from "@solana/web3.js";
// import { sendAndConfirmTransaction } from "@solana/web3.js";
// import { Program, Provider, AnchorProvider, web3 } from "@coral-xyz/anchor";
// import { defaultStaticAccounts } from "../constants";
// import { useWallet } from "../wallet";
// import { IDL, AccountCompression } from "../idls/account_compression";

// function byteArrayToKeypair(byteArray: number[]): Keypair {
//   return Keypair.fromSecretKey(Uint8Array.from(byteArray));
// }
// const MERKLE_TREE_TEST_KEYPAIR = byteArrayToKeypair([
//   146, 193, 80, 51, 114, 21, 221, 27, 228, 203, 43, 26, 211, 158, 183, 129, 254,
//   206, 249, 89, 121, 99, 123, 196, 106, 29, 91, 144, 50, 161, 42, 139, 68, 77,
//   125, 32, 76, 128, 61, 180, 1, 207, 69, 44, 121, 118, 153, 17, 179, 183, 115,
//   34, 163, 127, 102, 214, 1, 87, 175, 177, 95, 49, 65, 69,
// ]);
// const INDEXED_ARRAY_TEST_KEYPAIR = byteArrayToKeypair([
//   222, 130, 14, 179, 120, 234, 200, 231, 112, 214, 179, 171, 214, 95, 225, 61,
//   71, 61, 96, 214, 47, 253, 213, 178, 11, 77, 16, 2, 7, 24, 106, 218, 45, 107,
//   25, 100, 70, 71, 137, 47, 210, 248, 220, 223, 11, 204, 205, 89, 248, 48, 211,
//   168, 11, 25, 219, 158, 99, 47, 127, 248, 142, 107, 196, 110,
// ]);
// export const PAYER_KEYPAIR = byteArrayToKeypair([
//   17, 34, 231, 31, 83, 147, 93, 173, 61, 164, 25, 0, 204, 82, 234, 91, 202, 187,
//   228, 110, 146, 97, 112, 131, 180, 164, 96, 220, 57, 207, 65, 107, 2, 99, 226,
//   251, 88, 66, 92, 33, 25, 216, 211, 185, 112, 203, 212, 238, 105, 144, 72, 121,
//   176, 253, 106, 168, 115, 158, 154, 188, 62, 255, 166, 81,
// ]);

// export async function initMerkleTree() {
//   const connection = new Connection("http://localhost:8899");
//   const provider = new AnchorProvider(connection, useWallet(PAYER_KEYPAIR), {
//     preflightCommitment: "confirmed",
//   });
//   const accountCompressionProgram = defaultStaticAccounts()[2];
//   // FIXME: idl apparently broken
//   const program = new Program(IDL, accountCompressionProgram, provider);
//   //@ts-ignore

//   const merkleTreeKeypair = MERKLE_TREE_TEST_KEYPAIR;
//   // Get the minimum balance for rent exemption for the account
//   const rentExemption = 1_000_000_000_000; // 1000sol
//   const createAccountIx = SystemProgram.createAccount({
//     fromPubkey: PAYER_KEYPAIR.publicKey,
//     newAccountPubkey: merkleTreeKeypair.publicKey,
//     lamports: rentExemption,
//     space: 90368 + 128, // 104 // Adjust the space as necessary
//     programId: accountCompressionProgram,
//   });
// }

// export async function initIndexedArray() {}
