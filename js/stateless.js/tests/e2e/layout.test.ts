import { decodePublicTransactionEvent } from '../../src/programs/layout';
import { describe, it, expect } from 'vitest';
import { PublicKey } from '@solana/web3.js';
import BN from 'bn.js';
import { InstructionDataInvoke } from '../../src/state';

function deepEqual(ref: any, val: any) {
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

        for (let key of refKeys) {
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

describe('layout', () => {
    it('decode event', async () => {
        const data = [
            0, 0, 0, 0, 1, 0, 0, 0, 33, 32, 204, 221, 5, 83, 170, 139, 228, 191,
            81, 173, 10, 116, 229, 191, 155, 209, 23, 164, 28, 64, 188, 34, 248,
            127, 110, 97, 26, 188, 139, 164, 0, 0, 0, 0, 1, 0, 0, 0, 22, 143,
            135, 215, 254, 121, 58, 95, 241, 202, 91, 53, 255, 47, 224, 255, 67,
            218, 48, 172, 51, 208, 29, 102, 177, 187, 207, 73, 108, 18, 59, 255,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 1, 0, 0, 0, 68, 77, 125, 32, 76, 128, 61, 180, 1, 207, 69,
            44, 121, 118, 153, 17, 179, 183, 115, 34, 163, 127, 102, 214, 1, 87,
            175, 177, 95, 49, 65, 69, 0,
        ];

        const event = decodePublicTransactionEvent(Buffer.from(data));

        const refOutputCompressedAccountHash = [
            33, 32, 204, 221, 5, 83, 170, 139, 228, 191, 81, 173, 10, 116, 229,
            191, 155, 209, 23, 164, 28, 64, 188, 34, 248, 127, 110, 97, 26, 188,
            139, 164,
        ];

        expect(event.outputCompressedAccountHashes[0]).toEqual(
            Buffer.from(refOutputCompressedAccountHash),
        );
    });
    it('encode/decode CompressedTokenInstructionDataTransfer', () => {
        const data: InstructionDataInvoke = {};

        expect(deepEqual(decoded, data)).toBe(true);
    });
});
