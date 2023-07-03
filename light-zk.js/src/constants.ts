import * as anchor from "@coral-xyz/anchor";

import { ACCOUNT_DISCRIMINATOR_SIZE, Program } from "@coral-xyz/anchor";
import {
  ASSOCIATED_TOKEN_PROGRAM_ID,
  TOKEN_PROGRAM_ID,
} from "@solana/spl-token";

import {
  VerifierProgramTwo,
  VerifierProgramOne,
  VerifierProgramZero,
  MerkleTreeProgram,
} from "./idls/index";

import {
  ConfirmOptions,
  PublicKey,
  SystemProgram,
  SYSVAR_CLOCK_PUBKEY,
  SYSVAR_RENT_PUBKEY,
  TokenBalance,
} from "@solana/web3.js";
import { bs58 } from "@coral-xyz/anchor/dist/cjs/utils/bytes";
import { MINT, TokenData } from "./index";

export const CONSTANT_SECRET_AUTHKEY: Uint8Array = Uint8Array.from([
  155, 249, 234, 55, 8, 49, 0, 14, 84, 72, 10, 224, 21, 139, 87, 102, 115, 88,
  217, 72, 137, 38, 0, 179, 93, 202, 220, 31, 143, 79, 247, 200,
]);

export const FIELD_SIZE = new anchor.BN(
  "21888242871839275222246405745257275088548364400416034343698204186575808495617",
);

export const MERKLE_TREE_SIGNER_AUTHORITY = new PublicKey([
  59, 42, 227, 2, 155, 13, 249, 77, 6, 97, 72, 159, 190, 119, 46, 110, 226, 42,
  153, 232, 210, 107, 116, 255, 63, 213, 216, 18, 94, 128, 155, 225,
]);
export const TYPE_PUBKEY = { array: ["u8", 32] };
export const TYPE_SEED = { defined: "&[u8]" };
export const TYPE_INIT_DATA = { array: ["u8", 642] };

export const merkleTreeProgramId = new PublicKey(
  "JA5cjkRJ1euVi9xLWsCJVzsRzEkT8vcC4rqw9sVAo5d6",
);
export const verifierProgramStorageProgramId = new PublicKey(
  "DJpbogMSrK94E1zvvJydtkqoE4sknuzmMRoutd6B7TKj",
);
export const verifierProgramZeroProgramId = new PublicKey(
  "J1RRetZ4ujphU75LP8RadjXMf3sA12yC2R44CF7PmU7i",
);
export const verifierProgramOneProgramId = new PublicKey(
  "J85SuNBBsba7FQS66BiBCQjiQrQTif7v249zL2ffmRZc",
);
export const verifierProgramTwoProgramId = new PublicKey(
  "2cxC8e8uNYLcymH6RTGuJs3N8fXGkwmMpw45pY65Ay86",
);

export const LOOK_UP_TABLE = new PublicKey(
  "DyZnme4h32E66deCvsAV6pVceVw8s6ucRhNcwoofVCem",
);

export const MAX_U64 = new anchor.BN("18446744073709551615");
export const VERIFIER_PUBLIK_KEYS = [
  verifierProgramZeroProgramId,
  verifierProgramOneProgramId,
  verifierProgramTwoProgramId,
  verifierProgramStorageProgramId,
];
export type merkleTreeProgram = Program<MerkleTreeProgram>;
export type verifierProgramZero = Program<VerifierProgramZero>;
export type verifierProgramOne = Program<VerifierProgramOne>;
export type verifierProgramTwo = Program<VerifierProgramTwo>;

export const confirmConfig: ConfirmOptions = {
  commitment: "confirmed",
  preflightCommitment: "confirmed",
};

export const COMPRESSED_UTXO_BYTES_LENGTH = 96 + ACCOUNT_DISCRIMINATOR_SIZE;
export const ENCRYPTED_COMPRESSED_UTXO_BYTES_LENGTH =
  parseInt(((COMPRESSED_UTXO_BYTES_LENGTH + 16) / 16).toString()) * 16;
export const NACL_ENCRYPTED_COMPRESSED_UTXO_BYTES_LENGTH =
  COMPRESSED_UTXO_BYTES_LENGTH + 16;
export const UNCOMPRESSED_UTXO_BYTES_LENGTH =
  COMPRESSED_UTXO_BYTES_LENGTH + 2 * 32;
export const ENCRYPTED_UNCOMPRESSED_UTXO_BYTES_LENGTH =
  UNCOMPRESSED_UTXO_BYTES_LENGTH + 16;

export const DEFAULT_PRIVATE_KEY = bs58.encode(new Uint8Array(32).fill(0));

export const DEFAULT_ZERO =
  "14522046728041339886521211779101644712859239303505368468566383402165481390632";

export const AUTHORITY_SEED = anchor.utils.bytes.utf8.encode("AUTHORITY_SEED");
export const DEFAULT_PROGRAMS = {
  systemProgram: SystemProgram.programId,
  tokenProgram: TOKEN_PROGRAM_ID,
  associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
  rent: SYSVAR_RENT_PUBKEY,
  clock: SYSVAR_CLOCK_PUBKEY,
};

// recommented minimum amount of lamports to be able to pay for transaction fees
// needs to be more than 890_880 to be rentexempt
export const MINIMUM_LAMPORTS = new anchor.BN(890_880 + 150_000);

export const TOKEN_ACCOUNT_FEE = new anchor.BN(1_461_600 + 5000);

// TODO: make account object with important accounts
export const MESSAGE_MERKLE_TREE_KEY = new PublicKey(
  "65ugKwDCTmQvEAsEep842CjZArCmUQ2r37RW9VDLbNKy",
);
export const TRANSACTION_MERKLE_TREE_KEY = new PublicKey(
  "DCxUdYgqjE6AR9m13VvqpkxJqGJYnk8jn8NEeD3QY3BU",
);
export const REGISTERED_VERIFIER_PDA = new PublicKey(
  "Eo3jtUstuMCvapqXdWiYvoUJS1PJDtKVf6LdsMPdyoNn",
);
export const REGISTERED_VERIFIER_ONE_PDA = new PublicKey(
  "9Q5JQPJEqC71R3jTnrnrSEhjMouCVf2dNjURp1L25Wnr",
);
export const REGISTERED_VERIFIER_TWO_PDA = new PublicKey(
  "DRwtrkmoUe9VD4T2KRN2A41jqtHgdDeEH8b3sXu7dHVW",
);
export const AUTHORITY = new PublicKey(
  "KitaXMAzb8GPZcc6NW6mE7P6gV2fY3Bp8NqZWfeUwqM",
);
export const AUTHORITY_ONE = new PublicKey(
  "6n2eREPP6bMLLYVJSGcSCULFy7u2WDrx3v5GJR7bByMa",
);
export const PRE_INSERTED_LEAVES_INDEX = new PublicKey(
  "2MQ7XkirVZZhRQQKcaDiJsrXHCuRHjbu72sUEeW4eZjq",
);
export const TOKEN_AUTHORITY = new PublicKey(
  "GUqBxNbKyB9SBnbBKYR5dajwuWTjTRUhWrZgeFkJND55",
);
export const REGISTERED_POOL_PDA_SPL = new PublicKey(
  "2q4tXrgpsDffibmjfTGHU1gWCjYUfhwFnMyLX6dAhhr4",
);
export const REGISTERED_POOL_PDA_SPL_TOKEN = new PublicKey(
  "2mobV36eNyFGaMTKCHW1Jeoq64tUGuXqA4zGtY8SbxKh",
);
export const REGISTERED_POOL_PDA_SOL = new PublicKey(
  "Eti4Rjkx7ow88XkaFbxRStmwadTp8p9J2nSv7NhtuqDU",
);
export const POOL_TYPE = new Array(32).fill(0);
export const MERKLE_TREE_AUTHORITY_PDA = new PublicKey(
  "5EMc8sCbHeb1HtRFifcbCiXN66kX6Wddrd61EkdJun6Y",
);

export const TESTNET_LOOK_UP_TABLE = new PublicKey(
  "64Act4KKVEHFAnjaift46c4ZkutkmT4msN1esSnE6gaJ",
);

export const FEE_ASSET = anchor.web3.SystemProgram.programId;
export const MERKLE_TREE_HEIGHT = 18;
/** Threshold (per asset) at which new in-UTXOs get merged, in order to reduce UTXO pool size */
export const UTXO_MERGE_THRESHOLD = 20; // 7
export const UTXO_MERGE_MAXIMUM = 10;
export const UTXO_FEE_ASSET_MINIMUM = 100_000;
export const SIGN_MESSAGE: string =
  "IMPORTANT:\nThe application will be able to spend \nyour shielded assets. \n\nOnly sign the message if you trust this\n application.\n\n View all verified integrations here: \n'https://docs.lightprotocol.com/partners'";

export const RELAYER_FEES = 1e6;

// TODO: change once we have adapted getInstructions for repeating instructions
export const MAX_MESSAGE_SIZE = 800;

export const TOKEN_REGISTRY: Map<string, TokenData> = new Map([
  [
    "SOL",
    {
      symbol: "SOL",
      decimals: new anchor.BN(1e9),
      isNft: false, // TODO: parse from onchain state at configuration(decimlas, supply)
      isNative: true,
      mint: SystemProgram.programId,
    },
  ],
  [
    "USDC",
    {
      symbol: "USDC",
      decimals: new anchor.BN(1e2),
      isNft: false,
      isNative: false,
      // copied from MINT (test-utils)
      mint: new PublicKey("ycrF6Bw3doNPMSDmZM1rxNHimD2bwq1UFmifMCzbjAe"),
    },
  ],
]);

export const TOKEN_PUBKEY_SYMBOL: Map<string, string> = new Map([
  ["11111111111111111111111111111111", "SOL"],
  [
    "ycrF6Bw3doNPMSDmZM1rxNHimD2bwq1UFmifMCzbjAe", // is MINT in test-utils
    "USDC",
  ],
]);
