import { PublicKey } from '@solana/web3.js';
import { sha256 } from '@noble/hashes/sha256';
import { Buffer } from 'buffer';

const MAX_SEED_LENGTH = 32;
const MAX_SEEDS = 16;
const PDA_MARKER_BYTES = new TextEncoder().encode('ProgramDerivedAddress');

export function createProgramAddressSync(
    seeds: Array<Buffer | Uint8Array>,
    programId: PublicKey,
): PublicKey {
    if (typeof PublicKey.createProgramAddressSync === 'function') {
        return PublicKey.createProgramAddressSync(seeds, programId);
    }
    if (seeds.length > MAX_SEEDS) {
        throw new TypeError('Max number of seeds exceeded');
    }
    let buffer = Buffer.alloc(0);
    for (const seed of seeds) {
        if (seed.length > MAX_SEED_LENGTH) {
            throw new TypeError('Max seed length exceeded');
        }
        buffer = Buffer.concat([buffer, Buffer.from(seed)]);
    }
    const programIdBytes =
        typeof programId.toBuffer === 'function'
            ? programId.toBuffer()
            : Buffer.from(programId.toBytes());
    buffer = Buffer.concat([
        buffer,
        programIdBytes,
        Buffer.from(PDA_MARKER_BYTES),
    ]);
    const publicKeyBytes = sha256(new Uint8Array(buffer));
    if (PublicKey.isOnCurve(publicKeyBytes)) {
        throw new Error('Invalid seeds, address must fall off the curve');
    }
    return new PublicKey(publicKeyBytes);
}

export function findProgramAddressSync(
    seeds: Array<Buffer | Uint8Array>,
    programId: PublicKey,
): [PublicKey, number] {
    if (typeof PublicKey.findProgramAddressSync === 'function') {
        return PublicKey.findProgramAddressSync(seeds, programId);
    }
    let nonce = 255;
    while (nonce !== 0) {
        try {
            const seedsWithNonce = seeds.concat(Buffer.from([nonce]));
            const address = createProgramAddressSync(seedsWithNonce, programId);
            return [address, nonce];
        } catch (err) {
            if (err instanceof TypeError) {
                throw err;
            }
            nonce--;
        }
    }
    throw new Error('Unable to find a viable program address nonce');
}
