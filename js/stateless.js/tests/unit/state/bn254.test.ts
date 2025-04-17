import { describe, it, expect } from 'vitest';
import { createBN254, encodeBN254toBase58 } from '../../../src/state';
import { bn } from '../../../src/state';
import { PublicKey } from '@solana/web3.js';
import { FIELD_SIZE } from '../../../src/constants';

describe('createBN254 function', () => {
    it('should create a BN254 from a string', () => {
        const bigint = createBN254('100');
        expect(bigint.toNumber()).toBe(100);
    });

    it('should create a BN254 from a number', () => {
        const bigint = createBN254(100);
        expect(bigint.toNumber()).toBe(100);
    });

    it('should create a BN254 from a bigint', () => {
        const bigint = createBN254(bn(100));
        expect(bigint.toNumber()).toBe(100);
    });

    it('should create a BN254 from a Buffer', () => {
        const bigint = createBN254(Buffer.from([100]));
        expect(bigint.toNumber()).toBe(100);
    });

    it('should create a BN254 from a Uint8Array', () => {
        const bigint = createBN254(new Uint8Array([100]));
        expect(bigint.toNumber()).toBe(100);
    });

    it('should create a BN254 from a number[]', () => {
        const bigint = createBN254([100]);
        expect(bigint.toNumber()).toBe(100);
    });

    it('should create a BN254 from a base58 string', () => {
        const bigint = createBN254('2j', 'base58');
        expect(bigint.toNumber()).toBe(bn(100).toNumber());
    });
});

describe('encodeBN254toBase58 function', () => {
    it('should convert a BN254 to a base58 string, pad to 32 implicitly', () => {
        const bigint = createBN254('100');
        const base58 = encodeBN254toBase58(bigint);
        expect(base58).toBe('11111111111111111111111111111112j');
    });

    it('should match transformation via pubkey', () => {
        const refHash = [
            13, 225, 248, 105, 237, 121, 108, 70, 70, 197, 240, 130, 226, 236,
            129, 58, 213, 50, 236, 99, 216, 99, 91, 201, 141, 76, 196, 33, 41,
            181, 236, 187,
        ];
        const base58 = encodeBN254toBase58(bn(refHash));

        const pubkeyConv = new PublicKey(refHash).toBase58();
        expect(base58).toBe(pubkeyConv);
    });

    it('should pad to 32 bytes converting BN to Pubkey', () => {
        const refHash31 = [
            13, 225, 248, 105, 237, 121, 108, 70, 70, 197, 240, 130, 226, 236,
            129, 58, 213, 50, 236, 99, 216, 99, 91, 201, 141, 76, 196, 33, 41,
            181, 236,
        ];
        const base58 = encodeBN254toBase58(bn(refHash31));

        expect(
            createBN254(base58, 'base58').toArray('be', 32),
        ).to.be.deep.equal([0].concat(refHash31));
    });

    it('should throw an error for a value that is too large', () => {
        expect(() => createBN254(FIELD_SIZE)).toThrow(
            'Value is too large. Max <254 bits',
        );
    });
});
