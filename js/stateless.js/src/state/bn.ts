import BN from 'bn.js';
import { Buffer } from 'buffer';
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
