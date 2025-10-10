import BN from 'bn.js';
import { Buffer } from 'buffer';
export const bn = (
    number: string | number | BN | Buffer | Uint8Array | number[],
    base?: number | 'hex' | undefined,
    endian?: BN.Endianness | undefined,
): BN => {
    console.log('number', number);
    console.log('base', base);
    console.log('endian', endian);
    if (number instanceof Uint8Array && !(number instanceof Buffer)) {
        return new BN(Buffer.from(number), base, endian);
    }
    return new BN(number, base, endian);
};
