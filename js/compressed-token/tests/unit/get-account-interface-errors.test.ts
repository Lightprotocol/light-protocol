import { describe, expect, it } from 'vitest';
import { AccountInfo, PublicKey } from '@solana/web3.js';
import {
    AccountState,
    TOKEN_PROGRAM_ID,
    TokenInvalidAccountOwnerError,
    getAssociatedTokenAddressSync,
} from '@solana/spl-token';
import BN from 'bn.js';
import {
    LIGHT_TOKEN_PROGRAM_ID,
    featureFlags,
} from '@lightprotocol/stateless.js';
import { getAccountInterface, getAtaInterface } from '../../src/v3';
import { getAtaProgramId } from '../../src/v3/ata-utils';

const makeOnchainAccount = (
    owner: PublicKey,
    data: Buffer = Buffer.alloc(0),
): AccountInfo<Buffer> => ({
    executable: false,
    owner,
    lamports: 1,
    data,
    rentEpoch: 0,
});

const makeCompressedTokenData = (): Buffer => {
    const mint = PublicKey.unique().toBuffer();
    const owner = PublicKey.unique().toBuffer();
    const amount = new BN(42).toArrayLike(Buffer, 'le', 8);
    const delegateOption = Buffer.from([0]);
    const delegate = Buffer.alloc(32);
    const state = Buffer.from([AccountState.Initialized]);
    const tlvOption = Buffer.from([0]);
    return Buffer.concat([
        mint,
        owner,
        amount,
        delegateOption,
        delegate,
        state,
        tlvOption,
    ]);
};

const makeCompressedAccount = () =>
    ({
        owner: LIGHT_TOKEN_PROGRAM_ID,
        lamports: new BN(99),
        data: {
            discriminator: [1, 2, 3, 4, 5, 6, 7, 8],
            data: makeCompressedTokenData(),
            dataHash: new Array(32).fill(0),
        },
        treeInfo: {
            tree: PublicKey.unique(),
            queue: PublicKey.unique(),
            treeType: 2,
            nextTreeInfo: null,
        },
        hash: new BN(1234),
        leafIndex: 7,
        proveByIndex: false,
    }) as any;

describe.skipIf(!featureFlags.isV2() || !featureFlags.isBeta())(
    'get-account-interface errors',
    () => {
        it('throws TokenInvalidAccountOwnerError for light-token mode owner mismatch', async () => {
            const rpc = {
                getAccountInfo: async () =>
                    makeOnchainAccount(PublicKey.unique(), Buffer.alloc(10)),
                getCompressedTokenAccountsByOwner: async () => ({ items: [] }),
            } as any;

            await expect(
                getAccountInterface(
                    rpc,
                    PublicKey.unique(),
                    'confirmed',
                    LIGHT_TOKEN_PROGRAM_ID,
                ),
            ).rejects.toBeInstanceOf(TokenInvalidAccountOwnerError);
        });

        it('propagates SPL parse failures instead of downgrading to not-found', async () => {
            const rpc = {
                getAccountInfo: async () =>
                    makeOnchainAccount(TOKEN_PROGRAM_ID, Buffer.alloc(1)),
            } as any;

            await expect(
                getAccountInterface(
                    rpc,
                    PublicKey.unique(),
                    'confirmed',
                    TOKEN_PROGRAM_ID,
                ),
            ).rejects.toThrow('Failed to fetch token account data from RPC');
        });

        it('throws on RPC failures even when a compressed source exists', async () => {
            const rpc = {
                getAccountInfo: async () => {
                    throw new Error('solana down');
                },
                getCompressedTokenAccountsByOwner: async () => ({
                    items: [{ compressedAccount: makeCompressedAccount() }],
                }),
            } as any;

            await expect(
                getAccountInterface(
                    rpc,
                    PublicKey.unique(),
                    'confirmed',
                    LIGHT_TOKEN_PROGRAM_ID,
                ),
            ).rejects.toThrow(
                'Failed to fetch token account data from RPC: solana down',
            );
        });

        it('getAtaInterface propagates compressed RPC fetch failures', async () => {
            const owner = PublicKey.unique();
            const mint = PublicKey.unique();
            const ata = getAssociatedTokenAddressSync(
                mint,
                owner,
                false,
                LIGHT_TOKEN_PROGRAM_ID,
                getAtaProgramId(LIGHT_TOKEN_PROGRAM_ID),
            );

            const rpc = {
                getAccountInfo: async () => null,
                getCompressedTokenAccountsByOwner: async () => {
                    throw new Error('compression timeout');
                },
            } as any;

            await expect(
                getAtaInterface(
                    rpc,
                    ata,
                    owner,
                    mint,
                    'confirmed',
                    LIGHT_TOKEN_PROGRAM_ID,
                ),
            ).rejects.toThrow(
                'Failed to fetch token account data from RPC: compression timeout',
            );
        });
    },
);
