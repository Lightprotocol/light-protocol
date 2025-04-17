import BN from 'bn.js';
import { Buffer } from 'buffer';

// /**
//  * bignumber with <254-bit max size. Anchor serialization doesn't support native
//  * bigint yet, so we wrap BN. This wrapper has simple base10 encoding which is
//  * needed for zk circuit compat, in addition to the base58 encoding that users
//  * are used to from working with the web3.js PublicKey type.
//  */
// export type BN254 = BN;
export const bn = (
    number: string | number | BN | Buffer | Uint8Array | number[],
    base?: number | 'hex' | undefined,
    endian?: BN.Endianness | undefined,
): BN => {
    if (number instanceof Uint8Array && !(number instanceof Buffer)) {
        return new BN(Buffer.from(number), base, endian);
    }
    return new BN(number, base, endian);
};
