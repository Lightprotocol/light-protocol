import { describe, expect, it } from 'vitest';
import { AccountInfo, PublicKey } from '@solana/web3.js';
import { Rpc } from '../../../src/rpc';
import { featureFlags } from '../../../src/constants';
import BN from 'bn.js';

describe('getAccountInfoInterface', () => {
    const endpoint = 'http://127.0.0.1:8899';
    const itIfV2 = it.skipIf(!featureFlags.isV2());

    const makeRpc = () => new Rpc(endpoint, endpoint, endpoint);

    const makeOnchainAccount = (
        owner: PublicKey = PublicKey.unique(),
    ): AccountInfo<Buffer> => ({
        executable: false,
        owner,
        lamports: 123,
        data: Buffer.from([9, 9, 9]),
        rentEpoch: 0,
    });

    const makeCompressedAccount = () =>
        ({
            owner: PublicKey.unique(),
            lamports: new BN(456),
            data: {
                discriminator: [1, 2, 3, 4, 5, 6, 7, 8],
                data: Buffer.from([10, 11, 12]),
                dataHash: new Array(32).fill(0),
            },
            treeInfo: {
                tree: PublicKey.unique(),
                queue: PublicKey.unique(),
                treeType: 2,
                nextTreeInfo: null,
            },
            hash: new BN(999),
            leafIndex: 7,
            proveByIndex: false,
        }) as any;

    itIfV2(
        'returns null only when both sources are fulfilled and empty',
        async () => {
            const rpc = makeRpc();

            (rpc as any).getAccountInfo = async () => null;
            (rpc as any).getCompressedAccount = async () => null;

            const result = await rpc.getAccountInfoInterface(
                PublicKey.unique(),
                PublicKey.unique(),
            );

            expect(result).toBeNull();
        },
    );

    itIfV2(
        'propagates on-chain error when no definitive account is found',
        async () => {
            const rpc = makeRpc();
            const onchainError = new Error('solana rpc failed');

            (rpc as any).getAccountInfo = async () => {
                throw onchainError;
            };
            (rpc as any).getCompressedAccount = async () => null;

            await expect(
                rpc.getAccountInfoInterface(
                    PublicKey.unique(),
                    PublicKey.unique(),
                ),
            ).rejects.toThrow('solana rpc failed');
        },
    );

    itIfV2(
        'propagates compressed error when no definitive account is found',
        async () => {
            const rpc = makeRpc();
            const compressedError = new Error('compression rpc failed');

            (rpc as any).getAccountInfo = async () => null;
            (rpc as any).getCompressedAccount = async () => {
                throw compressedError;
            };

            await expect(
                rpc.getAccountInfoInterface(
                    PublicKey.unique(),
                    PublicKey.unique(),
                ),
            ).rejects.toThrow('compression rpc failed');
        },
    );

    itIfV2('throws on-chain error first when both sources reject', async () => {
        const rpc = makeRpc();
        const onchainError = new Error('solana rpc failed');
        const compressedError = new Error('compression rpc failed');

        (rpc as any).getAccountInfo = async () => {
            throw onchainError;
        };
        (rpc as any).getCompressedAccount = async () => {
            throw compressedError;
        };

        await expect(
            rpc.getAccountInfoInterface(PublicKey.unique(), PublicKey.unique()),
        ).rejects.toThrow('solana rpc failed');
    });

    itIfV2(
        'returns hot account when on-chain exists even if compressed call errors',
        async () => {
            const rpc = makeRpc();
            const hotAccount = makeOnchainAccount();

            (rpc as any).getAccountInfo = async () => hotAccount;
            (rpc as any).getCompressedAccount = async () => {
                throw new Error('compression rpc failed');
            };

            const result = await rpc.getAccountInfoInterface(
                PublicKey.unique(),
                PublicKey.unique(),
            );

            expect(result).not.toBeNull();
            expect(result!.isCold).toBe(false);
            expect(result!.accountInfo).toEqual(hotAccount);
            expect(result!.loadContext).toBeUndefined();
        },
    );

    itIfV2(
        'returns synthesized cold account when only compressed account exists',
        async () => {
            const rpc = makeRpc();
            const compressed = makeCompressedAccount();

            (rpc as any).getAccountInfo = async () => null;
            (rpc as any).getCompressedAccount = async () => compressed;

            const result = await rpc.getAccountInfoInterface(
                PublicKey.unique(),
                PublicKey.unique(),
            );

            expect(result).not.toBeNull();
            expect(result!.isCold).toBe(true);
            expect(result!.accountInfo.owner).toEqual(compressed.owner);
            expect(result!.accountInfo.lamports).toBe(456);
            expect(result!.accountInfo.data).toEqual(
                Buffer.concat([
                    Buffer.from(compressed.data.discriminator),
                    compressed.data.data,
                ]),
            );
            expect(result!.loadContext).toBeDefined();
            expect(result!.loadContext!.hash.toString()).toBe(
                compressed.hash.toString(),
            );
        },
    );
});
