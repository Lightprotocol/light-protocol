import { PublicKey } from '@solana/web3.js';
import {
    hashToBn254FieldSizeBe,
    hashvToBn254FieldSizeBe,
    hashvToBn254FieldSizeBeU8Array,
} from './conversion';
import { defaultTestStateTreeAccounts } from '../constants';
import { getIndexOrAdd } from '../programs/system/pack';
import { keccak_256 } from '@noble/hashes/sha3';

export function deriveAddressSeed(
    seeds: Uint8Array[],
    programId: PublicKey,
): Uint8Array {
    const combinedSeeds: Uint8Array[] = [programId.toBytes(), ...seeds];
    const hash = hashvToBn254FieldSizeBe(combinedSeeds);
    return hash;
}

/*
 * Derive an address for a compressed account from a seed and an address Merkle
 * tree public key.
 *
 * @param seed                     Seed to derive the address from
 * @param addressMerkleTreePubkey  Merkle tree public key. Defaults to
 *                                 defaultTestStateTreeAccounts().addressTree
 * @returns                        Derived address
 */
export function deriveAddress(
    seed: Uint8Array,
    addressMerkleTreePubkey: PublicKey = defaultTestStateTreeAccounts()
        .addressTree,
): PublicKey {
    if (seed.length != 32) {
        throw new Error('Seed length is not 32 bytes.');
    }
    const bytes = addressMerkleTreePubkey.toBytes();
    const combined = Buffer.from([...bytes, ...seed]);
    const hash = hashToBn254FieldSizeBe(combined);

    if (hash === null) {
        throw new Error('DeriveAddressError');
    }
    const buf = hash[0];
    return new PublicKey(buf);
}

export function deriveAddressSeedV2(seeds: Uint8Array[]): Uint8Array {
    const combinedSeeds: Uint8Array[] = seeds.map(seed =>
        Uint8Array.from(seed),
    );
    const hash = hashvToBn254FieldSizeBeU8Array(combinedSeeds);
    return hash;
}

/**
 * Derives an address from a seed using the v2 method (matching Rust's derive_address_from_seed)
 *
 * @param addressSeed              The address seed (32 bytes)
 * @param addressMerkleTreePubkey  Merkle tree public key
 * @param programId                Program ID
 * @returns                        Derived address
 */
export function deriveAddressV2(
    addressSeed: Uint8Array,
    addressMerkleTreePubkey: PublicKey,
    programId: PublicKey,
): PublicKey {
    if (addressSeed.length != 32) {
        throw new Error('Address seed length is not 32 bytes.');
    }
    const merkleTreeBytes = addressMerkleTreePubkey.toBytes();
    const programIdBytes = programId.toBytes();
    // Match Rust implementation: hash [seed, merkle_tree_pubkey, program_id]
    const combined = [
        Uint8Array.from(addressSeed),
        Uint8Array.from(merkleTreeBytes),
        Uint8Array.from(programIdBytes),
    ];
    const hash = hashvToBn254FieldSizeBeU8Array(combined);
    return new PublicKey(hash);
}

export interface NewAddressParams {
    /**
     * Seed for the compressed account. Must be seed used to derive
     * newAccountAddress
     */
    seed: Uint8Array;
    /**
     * Recent state root index of the address tree. The expiry is tied to the
     * validity proof.
     */
    addressMerkleTreeRootIndex: number;
    /**
     * Address tree pubkey. Must be base pubkey used to derive new address
     */
    addressMerkleTreePubkey: PublicKey;
    /**
     * Address space queue pubkey. Associated with the state tree.
     */
    addressQueuePubkey: PublicKey;
}

export interface NewAddressParamsPacked {
    /**
     * Seed for the compressed account. Must be seed used to derive
     * newAccountAddress
     */
    seed: number[];
    /**
     * Recent state root index of the address tree. The expiry is tied to the
     * validity proof.
     */
    addressMerkleTreeRootIndex: number;
    /**
     * Index of the address merkle tree account in the remaining accounts array
     */
    addressMerkleTreeAccountIndex: number;
    /**
     * Index of the address queue account in the remaining accounts array
     */
    addressQueueAccountIndex: number;
}

/**
 * Packs new address params for instruction data in TypeScript clients
 *
 * @param newAddressParams      New address params
 * @param remainingAccounts     Remaining accounts
 * @returns                     Packed new address params
 */
export function packNewAddressParams(
    newAddressParams: NewAddressParams[],
    remainingAccounts: PublicKey[],
): {
    newAddressParamsPacked: NewAddressParamsPacked[];
    remainingAccounts: PublicKey[];
} {
    const _remainingAccounts = remainingAccounts.slice();

    const newAddressParamsPacked: NewAddressParamsPacked[] =
        newAddressParams.map(x => ({
            seed: Array.from(x.seed),
            addressMerkleTreeRootIndex: x.addressMerkleTreeRootIndex,
            addressMerkleTreeAccountIndex: 0, // will be assigned later
            addressQueueAccountIndex: 0, // will be assigned later
        }));

    newAddressParams.forEach((params, i) => {
        newAddressParamsPacked[i].addressMerkleTreeAccountIndex = getIndexOrAdd(
            _remainingAccounts,
            params.addressMerkleTreePubkey,
        );
    });

    newAddressParams.forEach((params, i) => {
        newAddressParamsPacked[i].addressQueueAccountIndex = getIndexOrAdd(
            _remainingAccounts,
            params.addressQueuePubkey,
        );
    });

    return { newAddressParamsPacked, remainingAccounts: _remainingAccounts };
}

//@ts-ignore
if (import.meta.vitest) {
    //@ts-ignore
    const { it, expect, describe } = import.meta.vitest;

    const programId = new PublicKey(
        '7yucc7fL3JGbyMwg4neUaenNSdySS39hbAk89Ao3t1Hz',
    );

    describe('derive address seed', () => {
        it('should derive a valid address seed', () => {
            const seeds: Uint8Array[] = [
                new TextEncoder().encode('foo'),
                new TextEncoder().encode('bar'),
            ];
            expect(deriveAddressSeed(seeds, programId)).toStrictEqual(
                new Uint8Array([
                    0, 246, 150, 3, 192, 95, 53, 123, 56, 139, 206, 179, 253,
                    133, 115, 103, 120, 155, 251, 72, 250, 47, 117, 217, 118,
                    59, 174, 207, 49, 101, 201, 110,
                ]),
            );
        });

        it('should derive a valid address seed', () => {
            const seeds: Uint8Array[] = [
                new TextEncoder().encode('ayy'),
                new TextEncoder().encode('lmao'),
            ];
            expect(deriveAddressSeed(seeds, programId)).toStrictEqual(
                new Uint8Array([
                    0, 202, 44, 25, 221, 74, 144, 92, 69, 168, 38, 19, 206, 208,
                    29, 162, 53, 27, 120, 214, 152, 116, 15, 107, 212, 168, 33,
                    121, 187, 10, 76, 233,
                ]),
            );
        });
    });

    describe('deriveAddress function', () => {
        it('should derive a valid address from a seed and a merkle tree public key', async () => {
            const seeds: Uint8Array[] = [
                new TextEncoder().encode('foo'),
                new TextEncoder().encode('bar'),
            ];
            const seed = deriveAddressSeed(seeds, programId);
            const merkleTreePubkey = new PublicKey(
                '11111111111111111111111111111111',
            );
            const derivedAddress = deriveAddress(seed, merkleTreePubkey);
            expect(derivedAddress).toBeInstanceOf(PublicKey);
            expect(derivedAddress).toStrictEqual(
                new PublicKey('139uhyyBtEh4e1CBDJ68ooK5nCeWoncZf9HPyAfRrukA'),
            );
        });

        it('should derive a valid address from a seed and a merkle tree public key', async () => {
            const seeds: Uint8Array[] = [
                new TextEncoder().encode('ayy'),
                new TextEncoder().encode('lmao'),
            ];
            const seed = deriveAddressSeed(seeds, programId);
            const merkleTreePubkey = new PublicKey(
                '11111111111111111111111111111111',
            );
            const derivedAddress = deriveAddress(seed, merkleTreePubkey);
            expect(derivedAddress).toBeInstanceOf(PublicKey);
            expect(derivedAddress).toStrictEqual(
                new PublicKey('12bhHm6PQjbNmEn3Yu1Gq9k7XwVn2rZpzYokmLwbFazN'),
            );
        });
    });

    describe('packNewAddressParams function', () => {
        it('should pack new address params correctly', () => {
            const newAddressParams = [
                {
                    seed: new Uint8Array([1, 2, 3, 4]),
                    addressMerkleTreeRootIndex: 0,
                    addressMerkleTreePubkey: new PublicKey(
                        '11111111111111111111111111111111',
                    ),
                    addressQueuePubkey: new PublicKey(
                        '11111111111111111111111111111112',
                    ),
                },
            ];
            const remainingAccounts = [
                new PublicKey('11111111111111111111111111111112'),
                new PublicKey('11111111111111111111111111111111'),
            ];
            const packedParams = packNewAddressParams(
                newAddressParams,
                remainingAccounts,
            );
            expect(
                packedParams.newAddressParamsPacked[0]
                    .addressMerkleTreeAccountIndex,
            ).toBe(1);
            expect(
                packedParams.newAddressParamsPacked[0].addressQueueAccountIndex,
            ).toBe(0);
        });
    });
}
