import { PublicKey } from "@solana/web3.js";

/**
 * Custom program to load into LiteSVM
 */
export interface CustomProgram {
  /** Program ID */
  programId: PublicKey;
  /** Path to the program's .so file */
  programPath: string;
}

/**
 * Configuration options for LiteSVM test environment
 */
export interface LiteSVMConfig {
  /** Enable signature verification */
  sigverify?: boolean;
  /** Enable blockhash checking */
  blockhashCheck?: boolean;
  /** Initial lamports for the test environment */
  initialLamports?: bigint;
  /** Transaction history size */
  transactionHistorySize?: bigint;
  /** Custom programs to load */
  customPrograms?: CustomProgram[];
}
