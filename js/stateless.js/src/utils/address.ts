import { AccountMeta, PublicKey } from '@solana/web3.js';
import { hashToBn254FieldSizeBe } from './conversion';
import { defaultTestStateTreeAccounts } from '../constants';
import { getIndexOrAdd } from '../instruction';

/**
 * Derive an address for a compressed account from a seed and a merkle tree
 * public key.
 *
 * @param seed              Seed to derive the address from
 * @param merkleTreePubkey  Merkle tree public key. Defaults to
 *                          defaultTestStateTreeAccounts().merkleTree
 * @returns                 Derived address
 */
export async function deriveAddress(
    seed: Uint8Array,
    merkleTreePubkey: PublicKey = defaultTestStateTreeAccounts().merkleTree,
): Promise<PublicKey> {
    const bytes = merkleTreePubkey.toBytes();
    const combined = Buffer.from([...bytes, ...seed]);
    const hash = await hashToBn254FieldSizeBe(combined);

    if (hash === null) {
        throw new Error('DeriveAddressError');
    }
    const buf = hash[0];
    return new PublicKey(buf);
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

    describe('deriveAddress function', () => {
        it('should derive a valid address from a seed and a merkle tree public key', async () => {
            const seed = new Uint8Array([1, 2, 3, 4]);
            const merkleTreePubkey = new PublicKey(
                '11111111111111111111111111111111',
            );
            const derivedAddress = await deriveAddress(seed, merkleTreePubkey);
            expect(derivedAddress).toBeInstanceOf(PublicKey);
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
