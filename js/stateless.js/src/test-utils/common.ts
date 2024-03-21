import { Connection, Keypair, Signer } from '@solana/web3.js';
import { confirmTx } from '../utils';

let c = 1;

export const ALICE = getTestKeypair(255);
export const BOB = getTestKeypair(254);
export const CHARLIE = getTestKeypair(253);
export const DAVE = getTestKeypair(252);

export async function newAccountWithLamports(
    connection: Connection,
    lamports = 1000000000,
    counter: number | undefined = undefined,
): Promise<Signer> {
    const account = getTestKeypair(counter);
    const sig = await connection.requestAirdrop(account.publicKey, lamports);
    await confirmTx(connection, sig);
    return account;
}

export function getConnection(): Connection {
    const url = 'http://127.0.0.1:8899';
    const connection = new Connection(url, 'confirmed');
    return connection;
}

/**
 * For use in tests.
 * Generate a unique keypair by passing in a counter <255. If no counter
 * is supplied, it uses and increments a global counter.
 */
export function getTestKeypair(
    counter: number | undefined = undefined,
): Keypair {
    if (!counter) {
        counter = c;
        c++;
    }
    if (counter > 255) {
        throw new Error('Counter must be <= 255');
    }
    const seed = new Uint8Array(32);
    seed[0] = counter;

    return Keypair.fromSeed(seed);
}

if (import.meta.vitest) {
    const { describe, it, expect } = import.meta.vitest;

    describe('getTestKeypair', () => {
        it('should generate a keypair with a specific counter', () => {
            const keypair = getTestKeypair(10);
            expect(keypair).toBeInstanceOf(Keypair);
            expect(keypair.publicKey).toBeDefined();
            expect(keypair.secretKey).toBeDefined();
        });

        it('should throw an error if counter is greater than 255', () => {
            const testFn = () => getTestKeypair(256);
            expect(testFn).toThrow('Counter must be <= 255');
        });

        it('should increment the global counter if no counter is provided', () => {
            const initialKeypair = getTestKeypair();
            const nextKeypair = getTestKeypair();
            expect(initialKeypair).not.toEqual(nextKeypair);
        });
    });
}
