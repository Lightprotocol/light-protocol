import { describe, it, expect } from 'vitest';
import { PublicKey } from '@solana/web3.js';
import BN from 'bn.js';
import {
    CompressedTokenInstructionDataTransferLayout,
    CompressedTokenInstructionDataTransfer,
    mintToLayout,
    compressSplTokenAccountInstructionDataLayout,
    encodeMintToInstructionData,
    decodeMintToInstructionData,
    encodeCompressSplTokenAccountInstructionData,
    decodeCompressSplTokenAccountInstructionData,
    encodeTransferInstructionData,
    decodeTransferInstructionData,
    MintToInstructionData,
    CompressSplTokenAccountInstructionData,
} from '../../src/';

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
    it('encode/decode CompressedTokenInstructionDataTransfer', () => {
        const data: CompressedTokenInstructionDataTransfer = {
            proof: null,
            mint: new PublicKey('CXzk7xBgfzwSrZincWbJPGMPFz8aut3V6JjXp5n3XvGQ'),
            delegatedTransfer: null,
            inputTokenDataWithContext: [],
            outputCompressedAccounts: [
                {
                    owner: new PublicKey(
                        '7gpbzzu2Aj2sE7LFdEQKUoZLqxVAK3eQ9LoeKQ5zCxQJ',
                    ),
                    amount: new BN(700),
                    lamports: null,
                    merkleTreeIndex: 0,
                    tlv: null,
                },
            ],
            isCompress: true,
            compressOrDecompressAmount: new BN(700),
            cpiContext: null,
            lamportsChangeAccountMerkleTreeIndex: null,
        };

        const encoded = encodeTransferInstructionData(data);
        const decoded = decodeTransferInstructionData(encoded);
        expect(deepEqual(decoded, data)).toBe(true);
    });

    it('encode/decode MintToInstructionData', () => {
        const data = {
            recipients: [
                new PublicKey('CXzk7xBgfzwSrZincWbJPGMPFz8aut3V6JjXp5n3XvGQ'),
            ],
            amounts: [new BN(1000)],
            lamports: new BN(500),
        };

        const encoded = encodeMintToInstructionData(data);
        const decoded = decodeMintToInstructionData(encoded);
        expect(deepEqual(decoded, data)).toBe(true);
    });

    it('encode/decode CompressSplTokenAccountInstructionData', () => {
        const data = {
            owner: new PublicKey(
                'CXzk7xBgfzwSrZincWbJPGMPFz8aut3V6JjXp5n3XvGQ',
            ),
            remainingAmount: new BN(1000),
            cpiContext: null,
        };

        const encoded = encodeCompressSplTokenAccountInstructionData(data);
        const decoded = decodeCompressSplTokenAccountInstructionData(encoded);
        expect(deepEqual(decoded, data)).toBe(true);
    });

    it('validate CompressedTokenInstructionDataTransferLayout', () => {
        const data: CompressedTokenInstructionDataTransfer = {
            proof: null,
            mint: new PublicKey('CXzk7xBgfzwSrZincWbJPGMPFz8aut3V6JjXp5n3XvGQ'),
            delegatedTransfer: null,
            inputTokenDataWithContext: [],
            outputCompressedAccounts: [
                {
                    owner: new PublicKey(
                        '7gpbzzu2Aj2sE7LFdEQKUoZLqxVAK3eQ9LoeKQ5zCxQJ',
                    ),
                    amount: new BN(700),
                    lamports: null,
                    merkleTreeIndex: 0,
                    tlv: null,
                },
            ],
            isCompress: true,
            compressOrDecompressAmount: new BN(700),
            cpiContext: null,
            lamportsChangeAccountMerkleTreeIndex: null,
        };

        const buffer = Buffer.alloc(1000);
        const len = CompressedTokenInstructionDataTransferLayout.encode(
            data,
            buffer,
        );
        const decoded = CompressedTokenInstructionDataTransferLayout.decode(
            buffer.slice(0, len),
        );
        expect(deepEqual(decoded, data)).toBe(true);
    });

    it('validate mintToLayout', () => {
        const data: MintToInstructionData = {
            recipients: [
                new PublicKey('CXzk7xBgfzwSrZincWbJPGMPFz8aut3V6JjXp5n3XvGQ'),
            ],
            amounts: [new BN(1000)],
            lamports: new BN(500),
        };

        const buffer = Buffer.alloc(1000);
        const len = mintToLayout.encode(data, buffer);
        const decoded = mintToLayout.decode(buffer.slice(0, len));
        expect(deepEqual(decoded, data)).toBe(true);
    });

    it('validate compressSplTokenAccountInstructionDataLayout', () => {
        const data: CompressSplTokenAccountInstructionData = {
            owner: new PublicKey(
                'CXzk7xBgfzwSrZincWbJPGMPFz8aut3V6JjXp5n3XvGQ',
            ),
            remainingAmount: new BN(1000),
            cpiContext: null,
        };

        const buffer = Buffer.alloc(1000);
        const len = compressSplTokenAccountInstructionDataLayout.encode(
            data,
            buffer,
        );
        const decoded = compressSplTokenAccountInstructionDataLayout.decode(
            buffer.slice(0, len),
        );
        expect(deepEqual(decoded, data)).toBe(true);
    });
});
