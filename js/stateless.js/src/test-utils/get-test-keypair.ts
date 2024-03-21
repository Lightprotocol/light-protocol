import { Keypair } from '@solana/web3.js';

let c = 1;

export const ALICE = getTestKeypair(255);
export const BOB = getTestKeypair(254);
export const CHARLIE = getTestKeypair(253);
export const DAVE = getTestKeypair(252);

export const TEST_MINT_1 = getTestKeypair(251);
export const TEST_MINT_2 = getTestKeypair(250);

/**
 * For use in tests.
 * Generate a unique keypair by passing in a counter <255. If no counter
 * is supplied, it uses and increments a global counter.
 */
export function getTestKeypair(counter: number = c): Keypair {
    if (counter > 255) {
        throw new Error('Counter must be <= 255');
    }
    const seed = new Uint8Array(32);
    seed[0] = counter++;
    return Keypair.fromSeed(seed);
}
