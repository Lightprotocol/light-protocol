import { Connection, Keypair, Signer } from '@solana/web3.js';
import { confirmTx } from '../utils/send-and-confirm';
import { Rpc } from '../rpc';
import BN from 'bn.js';

let c = 1;

export const ALICE = getTestKeypair(255);
export const BOB = getTestKeypair(254);
export const CHARLIE = getTestKeypair(253);
export const DAVE = getTestKeypair(252);

/**
 * Deep comparison of two objects. Handles BN comparison correctly.
 *
 * @param ref - The reference object to compare.
 * @param val - The value object to compare.
 * @returns True if the objects are deeply equal, false otherwise.
 */
export function deepEqual(ref: any, val: any) {
    if (typeof ref !== typeof val) {
        console.log(`Type mismatch: ${typeof ref} !== ${typeof val}`);
        return false;
    }

    if (ref instanceof BN && val instanceof BN) {
        return ref.eq(val);
    }

    if (typeof ref === 'object' && ref !== null && val !== null) {
        const refKeys = Object.keys(ref);
        const valKeys = Object.keys(val);

        if (refKeys.length !== valKeys.length) {
            console.log(
                `Key length mismatch: ${refKeys.length} !== ${valKeys.length}`,
            );
            return false;
        }

        for (const key of refKeys) {
            if (!valKeys.includes(key)) {
                console.log(`Key ${key} not found in value`);
                return false;
            }
            if (!deepEqual(ref[key], val[key])) {
                console.log(`Value mismatch at key ${key}`);
                return false;
            }
        }
        return true;
    }

    if (ref !== val) {
        console.log(`Value mismatch: ${ref} !== ${val}`);
    }

    return ref === val;
}

/**
 * Create a new account and airdrop lamports to it
 *
 * @param rpc       connection to use
 * @param lamports  amount of lamports to airdrop
 * @param counter   counter to use for generating the keypair.
 *                  If undefined or >255, generates random keypair.
 */
export async function newAccountWithLamports(
    rpc: Rpc,
    lamports = 1000000000,
    counter: number | undefined = undefined,
): Promise<Signer> {
    /// get random keypair
    if (counter === undefined || counter > 255) {
        counter = 256;
    }

    const account = getTestKeypair(counter);
    const sig = await rpc.requestAirdrop(account.publicKey, lamports);
    await confirmTx(rpc, sig);
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
 * if counter > 255, generates random keypair
 */
export function getTestKeypair(
    counter: number | undefined = undefined,
): Keypair {
    if (!counter) {
        counter = c;
        c++;
    }
    if (counter > 255) {
        return Keypair.generate();
    }
    const seed = new Uint8Array(32);
    seed[31] = counter; // le

    return Keypair.fromSeed(seed);
}

//@ts-ignore
if (import.meta.vitest) {
    //@ts-ignore
    const { describe, it, expect } = import.meta.vitest;

    describe('getTestKeypair', () => {
        it('should generate a keypair with a specific counter', () => {
            const keypair = getTestKeypair(10);
            const keypair2 = getTestKeypair(10);
            expect(keypair).toEqual(keypair2);
            expect(keypair).toBeInstanceOf(Keypair);
            expect(keypair.publicKey).toBeDefined();
            expect(keypair.secretKey).toBeDefined();
        });

        it('should generate random keypair if counter is greater than 255', () => {
            const testFn = () => getTestKeypair(256);
            const kp1 = testFn();
            const kp2 = testFn();
            expect(kp1).not.toEqual(kp2);
        });

        it('should increment the global counter if no counter is provided', () => {
            const initialKeypair = getTestKeypair();
            const nextKeypair = getTestKeypair();
            const nextNextKeypair = getTestKeypair();
            const nextNextNextKeypair = getTestKeypair(3);
            expect(initialKeypair).not.toEqual(nextKeypair);
            expect(nextKeypair).not.toEqual(nextNextKeypair);
            expect(nextNextKeypair).toEqual(nextNextNextKeypair);
        });
    });
}
