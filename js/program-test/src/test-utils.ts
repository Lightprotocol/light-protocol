import { Keypair, PublicKey, LAMPORTS_PER_SOL } from "@solana/web3.js";
import { LiteSVMRpc } from "./litesvm-rpc";

/**
 * Create a new account with lamports airdropped
 */
export async function newAccountWithLamports(
  rpc: LiteSVMRpc,
  lamports: number = LAMPORTS_PER_SOL,
): Promise<Keypair> {
  const keypair = Keypair.generate();
  await rpc.requestAirdrop(keypair.publicKey, lamports);
  return keypair;
}

/**
 * Sleep for a specified duration (useful for test delays)
 */
export async function sleep(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

/**
 * Get or create a keypair from optional seed
 */
export function getOrCreateKeypair(seed?: Uint8Array): Keypair {
  if (seed) {
    return Keypair.fromSeed(seed.slice(0, 32));
  }
  return Keypair.generate();
}
