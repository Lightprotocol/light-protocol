import { describe, expect, it } from 'vitest';
import { Commitment, PublicKey } from '@solana/web3.js';
import {
    LIGHT_TOKEN_PROGRAM_ID,
    featureFlags,
} from '@lightprotocol/stateless.js';
import {
    TokenAccountNotFoundError,
    TokenInvalidAccountOwnerError,
} from '@solana/spl-token';
import { getMintInterface } from '../../src/v3/get-mint-interface';

const makeCompressedMintSentinelAccount = () =>
    ({
        owner: LIGHT_TOKEN_PROGRAM_ID,
        data: {
            discriminator: [1, 2, 3, 4, 5, 6, 7, 8],
            // < 64 bytes means "decompressed" sentinel branch.
            data: Buffer.alloc(32),
            dataHash: new Array(32).fill(0),
        },
        treeInfo: {
            tree: PublicKey.unique(),
            queue: PublicKey.unique(),
            treeType: 2,
            nextTreeInfo: null,
        },
        hash: Buffer.alloc(32),
        leafIndex: 0,
        proveByIndex: false,
        lamports: { toNumber: () => 0 },
    }) as any;

describe.skipIf(!featureFlags.isV2() || !featureFlags.isBeta())(
    'get-mint-interface errors',
    () => {
        it('throws TokenInvalidAccountOwnerError when decompressed on-chain mint owner mismatches', async () => {
            const mint = PublicKey.unique();
            const rpc = {
                getCompressedAccount: async () =>
                    makeCompressedMintSentinelAccount(),
                getAccountInfo: async () => ({
                    executable: false,
                    owner: PublicKey.unique(),
                    lamports: 1,
                    data: Buffer.alloc(100),
                    rentEpoch: 0,
                }),
            } as any;

            await expect(
                getMintInterface(
                    rpc,
                    mint,
                    'confirmed',
                    LIGHT_TOKEN_PROGRAM_ID,
                ),
            ).rejects.toBeInstanceOf(TokenInvalidAccountOwnerError);
        });

        it('forwards commitment when fetching decompressed on-chain mint', async () => {
            const mint = PublicKey.unique();
            const commitment: Commitment = 'processed';
            let seenCommitment: Commitment | undefined;

            const rpc = {
                getCompressedAccount: async () =>
                    makeCompressedMintSentinelAccount(),
                getAccountInfo: async (
                    _address: PublicKey,
                    incomingCommitment?: Commitment,
                ) => {
                    seenCommitment = incomingCommitment;
                    return null;
                },
            } as any;

            await expect(
                getMintInterface(rpc, mint, commitment, LIGHT_TOKEN_PROGRAM_ID),
            ).rejects.toBeInstanceOf(TokenAccountNotFoundError);
            expect(seenCommitment).toBe(commitment);
        });
    },
);
