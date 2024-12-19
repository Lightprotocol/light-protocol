import { describe, it, expect } from 'vitest';
import { PublicTransactionEvent, bn } from '../../src';
import {
    CompressedAccountLayout,
    decodePublicTransactionEvent,
} from '../../src/programs/layout';

describe('serde', () => {
    // it('decode output compressed account', async () => {
    //     const compressedAccount = [
    //         88, 8, 48, 185, 124, 227, 14, 195, 230, 152, 61, 39, 56, 191, 13,
    //         126, 54, 43, 47, 131, 175, 16, 52, 167, 129, 174, 200, 118, 174, 9,
    //         254, 80, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    //     ];

    //     const deserializedCompressedAccount = CompressedAccountLayout.decode(
    //         Buffer.from(compressedAccount),
    //     );

    //     expect(deserializedCompressedAccount.data).toBe(null);
    //     expect(deserializedCompressedAccount.address).toBe(null);
    //     expect(deserializedCompressedAccount.lamports.eq(bn(0))).toBe(true);
    // });

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
});
