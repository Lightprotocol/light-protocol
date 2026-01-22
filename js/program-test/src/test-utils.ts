import { Keypair, PublicKey, LAMPORTS_PER_SOL } from "@solana/web3.js";
import { Rpc } from "@lightprotocol/stateless.js";

/**
 * Create a new account with lamports airdropped
 */
export async function newAccountWithLamports(
  rpc: Rpc,
  lamports: number = LAMPORTS_PER_SOL,
): Promise<Keypair> {
  const keypair = Keypair.generate();
  const signature = await rpc.requestAirdrop(keypair.publicKey, lamports);
  await rpc.confirmTransaction(signature);
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
