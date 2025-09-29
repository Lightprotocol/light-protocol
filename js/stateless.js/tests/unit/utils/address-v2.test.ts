import { describe, it, expect } from 'vitest';
import { PublicKey } from '@solana/web3.js';
import {
    deriveAddressSeedV2,
    deriveAddressV2,
} from '../../../src/utils/address';

describe('V2 Address Derivation - Rust Compatibility Tests', () => {
    const programId = new PublicKey(
        '7yucc7fL3JGbyMwg4neUaenNSdySS39hbAk89Ao3t1Hz',
    );
    const addressTreePubkey = new PublicKey(new Uint8Array(32).fill(0));

    // Test vectors from Rust implementation
    const testCases = [
        {
            name: '["foo", "bar"]',
            seeds: ['foo', 'bar'],
            expectedSeed: [
                0, 177, 134, 198, 24, 76, 116, 207, 56, 127, 189, 181, 87, 237,
                154, 181, 246, 54, 131, 21, 150, 248, 106, 75, 26, 80, 147, 245,
                3, 23, 136, 56,
            ],
            expectedAddress: [
                0, 16, 227, 141, 38, 32, 23, 82, 252, 50, 202, 3, 183, 186, 236,
                133, 86, 112, 59, 23, 128, 162, 11, 84, 91, 127, 179, 208, 25,
                178, 1, 240,
            ],
        },
        {
            name: '["ayy", "lmao"]',
            seeds: ['ayy', 'lmao'],
            expectedSeed: [
                0, 224, 206, 65, 137, 189, 70, 157, 163, 133, 247, 140, 198,
                252, 169, 250, 18, 18, 16, 189, 164, 131, 225, 113, 197, 225,
                64, 81, 175, 154, 221, 28,
            ],
            expectedAddress: [
                0, 226, 28, 142, 199, 153, 126, 212, 37, 54, 82, 232, 244, 161,
                108, 12, 67, 84, 111, 66, 107, 111, 8, 126, 153, 233, 239, 192,
                83, 117, 25, 6,
            ],
        },
    ];

    describe('deriveAddressSeedV2', () => {
        testCases.forEach(({ name, seeds, expectedSeed }) => {
            it(`should match Rust for ${name}`, () => {
                const seedBytes = seeds.map(s => new TextEncoder().encode(s));
                const addressSeed = deriveAddressSeedV2(seedBytes);
                expect(addressSeed).toStrictEqual(new Uint8Array(expectedSeed));
            });
        });
    });

    describe('deriveAddressV2', () => {
        testCases.forEach(({ name, seeds, expectedSeed, expectedAddress }) => {
            it(`should match Rust for ${name}`, () => {
                const seedBytes = seeds.map(s => new TextEncoder().encode(s));
                const addressSeed = deriveAddressSeedV2(seedBytes);

                expect(addressSeed).toStrictEqual(new Uint8Array(expectedSeed));

                const derivedAddress = deriveAddressV2(
                    addressSeed,
                    addressTreePubkey,
                    programId,
                );

                expect(derivedAddress.toBytes()).toStrictEqual(
                    new Uint8Array(expectedAddress),
                );
            });
        });
    });
});
