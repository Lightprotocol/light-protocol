import { describe, it, expect } from 'vitest';
import { PublicKey } from '@solana/web3.js';
import BN from 'bn.js';
import {
    CompressedTokenInstructionDataTransferLayout,
    TRANSFER_DISCRIMINATOR,
} from '../../src/layout';
import { CompressedTokenInstructionDataTransfer } from '../../../stateless.js/src';

describe('layout', () => {
    it('encode CompressedTokenInstructionDataTransfer', () => {
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

        const lengthBuffer = Buffer.alloc(4);
        lengthBuffer.writeUInt32LE(len, 0);

        const encoded = Buffer.concat([
            TRANSFER_DISCRIMINATOR,
            lengthBuffer,
            buffer.slice(0, len),
        ]);

        expect(encoded.length).toBeGreaterThan(0);
        expect(encoded.slice(0, 8)).toEqual(TRANSFER_DISCRIMINATOR);
        expect(Array.from(encoded.slice(0, -8))).toEqual([
            163, 52, 200, 231, 140, 3, 69, 186, 97, 0, 0, 0, 0, 171, 97, 69,
            108, 86, 192, 2, 185, 79, 47, 215, 58, 29, 81, 87, 205, 244, 20,
            157, 24, 221, 133, 48, 179, 204, 116, 15, 94, 62, 203, 8, 97, 0, 0,
            0, 0, 0, 1, 0, 0, 0, 99, 89, 153, 55, 109, 114, 208, 114, 149, 17,
            69, 25, 230, 251, 164, 56, 142, 112, 116, 91, 104, 218, 126, 175,
            171, 134, 147, 64, 101, 207, 16, 139, 188, 2, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 1, 1, 188,
        ]);
    });
});
