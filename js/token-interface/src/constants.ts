import { Buffer } from 'buffer';
import { PublicKey } from '@solana/web3.js';

export const LIGHT_TOKEN_CONFIG = new PublicKey(
    'ACXg8a7VaqecBWrSbdu73W4Pg9gsqXJ3EXAqkHyhvVXg',
);

export const LIGHT_TOKEN_RENT_SPONSOR = new PublicKey(
    'r18WwUxfG8kQ69bQPAB2jV6zGNKy3GosFGctjQoV4ti',
);

export enum TokenDataVersion {
    V1 = 1,
    V2 = 2,
    ShaFlat = 3,
}

export const POOL_SEED = Buffer.from('pool');
export const CPI_AUTHORITY_SEED = Buffer.from('cpi_authority');
export const MAX_TOP_UP = 65535;

export const COMPRESSED_TOKEN_PROGRAM_ID = new PublicKey(
    'cTokenmWW8bLPjZEBAUgYy3zKxQZW6VKi7bqNFEVv3m',
);

export function deriveSplPoolPdaWithIndex(
    mint: PublicKey,
    index: number,
): [PublicKey, number] {
    const indexSeed = index === 0 ? Buffer.from([]) : Buffer.from([index & 0xff]);
    return PublicKey.findProgramAddressSync(
        [POOL_SEED, mint.toBuffer(), indexSeed],
        COMPRESSED_TOKEN_PROGRAM_ID,
    );
}

export function deriveCpiAuthorityPda(): PublicKey {
    return PublicKey.findProgramAddressSync(
        [CPI_AUTHORITY_SEED],
        COMPRESSED_TOKEN_PROGRAM_ID,
    )[0];
}
