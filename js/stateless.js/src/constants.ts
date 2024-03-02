import { Program, Idl } from "@coral-xyz/anchor";
import { IDL as IDL_PSP_COMPRESSED_PDA } from "./idls/psp_compressed_pda";

import {
  ConfirmOptions,
  PublicKey,
  SystemProgram,
  SYSVAR_CLOCK_PUBKEY,
  SYSVAR_RENT_PUBKEY,
} from "@solana/web3.js";
import { utf8 } from "@coral-xyz/anchor/dist/cjs/utils/bytes";

export const FIELD_SIZE = BigInt(
  "21888242871839275222246405745257275088548364400416034343698204186575808495617"
);

// TODO: implement properly
export const noopProgram = "noopb9bkMVfRPU8AsbpTUg8AQkHtKwMYZiFUjNRtMmV";
export const accountCompressionProgram =
  "noopb9bkMVfRPU8AsbpTUg8AQkHtKwMYZiFUjNRtMmV";
export const registeredProgramPda =
  "noopb9bkMVfRPU8AsbpTUg8AQkHtKwMYZiFUjNRtMmV";
export const accountCompressionAuthority =
  "noopb9bkMVfRPU8AsbpTUg8AQkHtKwMYZiFUjNRtMmV";
export const cpiSignatureAccount =
  "noopb9bkMVfRPU8AsbpTUg8AQkHtKwMYZiFUjNRtMmV";

export const defaultStaticAccounts = () => [
  new PublicKey(registeredProgramPda),
  new PublicKey(noopProgram),
  new PublicKey(accountCompressionProgram),
  new PublicKey(accountCompressionAuthority),
  new PublicKey(cpiSignatureAccount),
];

export const TYPE_PUBKEY = { array: ["u8", 32] };
export const TYPE_SEED = { defined: "&[u8]" };
export const TYPE_INIT_DATA = { array: ["u8", 642] };
export const MAX_U64 = BigInt("18446744073709551615");
export const AUTHORITY_SEED = utf8.encode("AUTHORITY_SEED");

/// TODO: replace mock
export const merkleTreeProgramId = new PublicKey(
  "JA5cjkRJ1euVi9xLWsCJVzsRzEkT8vcC4rqw9sVAo5d6"
);
/// TODO: replace mock
export const LOOK_UP_TABLE = new PublicKey(
  "DyZnme4h32E66deCvsAV6pVceVw8s6ucRhNcwoofVCem"
);

/// TODO: replace mock
export type merkleTreeProgram = Program<null>;

/// TODO: replace mock
export const MERKLE_TREE_SIGNER_AUTHORITY = new PublicKey([
  59, 42, 227, 2, 155, 13, 249, 77, 6, 97, 72, 159, 190, 119, 46, 110, 226, 42,
  153, 232, 210, 107, 116, 255, 63, 213, 216, 18, 94, 128, 155, 225,
]);

export const confirmConfig: ConfirmOptions = {
  commitment: "confirmed",
  preflightCommitment: "confirmed",
};

export const LIGHT_SYSTEM_PROGRAMS = {
  pspCompressedPda: PublicKey.default, /// TODO: replace with actual programId
};
export const SOLANA_DEFAULT_PROGRAMS = {
  systemProgram: SystemProgram.programId,
  rent: SYSVAR_RENT_PUBKEY,
  clock: SYSVAR_CLOCK_PUBKEY,
};

/// TODO: replace mock
export const MERKLE_TREE_AUTHORITY_PDA = new PublicKey(
  "5EMc8sCbHeb1HtRFifcbCiXN66kX6Wddrd61EkdJun6Y"
);
/// TODO: replace mock
export const MERKLE_TREE_SET = new PublicKey(
  "BrY8P3ZuLWFptfY7qwvkRZkEaD88UEByz9wKRuXFEwhr"
);
/// TODO: replace mock
export const TESTNET_LOOK_UP_TABLE = new PublicKey(
  "64Act4KKVEHFAnjaift46c4ZkutkmT4msN1esSnE6gaJ"
);

export const FEE_ASSET = SystemProgram.programId;
export const DEFAULT_MERKLE_TREE_HEIGHT = 22;
export const DEFAULT_MERKLE_TREE_ROOTS = 2800;
/** Threshold (per asset) at which new in-UTXOs get merged, in order to reduce UTXO pool size */
export const UTXO_MERGE_THRESHOLD = 20;
export const UTXO_MERGE_MAXIMUM = 10;
export const COMPRESSED_LAMPORTS_MINIMUM = 0;
export const DEFAULT_RELAY_FEE = BigInt(0);

/**
 * Treshold after which the currently used transaction Merkle tree is switched
 * to the next one
 */
export const TRANSACTION_MERKLE_TREE_ROLLOVER_THRESHOLD = BigInt(
  Math.floor(2 ** DEFAULT_MERKLE_TREE_HEIGHT * 0.95)
);
// @ts-ignore: anchor type error for different idls figure out whether we can avoid
export const LIGHT_SYSTEM_PROGRAM_IDLS: Map<string, Idl> = new Map([
  [LIGHT_SYSTEM_PROGRAMS.pspCompressedPda.toBase58(), IDL_PSP_COMPRESSED_PDA],
]);
