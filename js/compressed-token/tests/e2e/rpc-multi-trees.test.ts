import { describe, it, beforeAll, expect } from 'vitest';
import { Keypair, PublicKey, Signer } from '@solana/web3.js';
import {
    Rpc,
    newAccountWithLamports,
    bn,
    createRpc,
    StateTreeInfo,
    TreeType,
} from '@lightprotocol/stateless.js';
import { createMint, mintTo, transfer } from '../../src/actions';
import { getStateTreeInfoByTypeForTest } from '../../../stateless.js/tests/e2e/shared';

const TEST_TOKEN_DECIMALS = 2;

describe.each([TreeType.StateV1])(
    'rpc-multi-trees with state tree %s',
    treeType => {
        let rpc: Rpc;

        let payer: Signer;
        let bob: Signer;
        let charlie: Signer;
        let mint: PublicKey;
        let mintAuthority: Keypair;
        let outputStateTreeInfo: StateTreeInfo;
        let outputStateTreeInfoV2: StateTreeInfo;

        beforeAll(async () => {
            rpc = createRpc();

            outputStateTreeInfo = await getStateTreeInfoByTypeForTest(
                rpc,
                treeType,
            );
            outputStateTreeInfoV2 = await getStateTreeInfoByTypeForTest(
                rpc,
                treeType,
            );

            payer = await newAccountWithLamports(rpc, 1e9, 252);
            mintAuthority = Keypair.generate();
            const mintKeypair = Keypair.generate();

            mint = (
                await createMint(
                    rpc,
                    payer,
                    mintAuthority.publicKey,
                    TEST_TOKEN_DECIMALS,
                    mintKeypair,
                )
            ).mint;

            bob = await newAccountWithLamports(rpc, 1e9, 256);
            charlie = await newAccountWithLamports(rpc, 1e9, 256);

            await mintTo(
                rpc,
                payer,
                mint,
                bob.publicKey,
                mintAuthority,
                bn(1000),
                outputStateTreeInfo,
            );

            // should auto land in same tree
            await transfer(
                rpc,
                payer,
                mint,
                bn(700),
                bob,
                charlie.publicKey,
                outputStateTreeInfoV2,
            );
        });

        it('getCompressedTokenAccountsByOwner work with random state tree', async () => {
            const senderAccounts = (
                await rpc.getCompressedTokenAccountsByOwner(bob.publicKey, {
                    mint,
                })
            ).items;

            const receiverAccounts = (
                await rpc.getCompressedTokenAccountsByOwner(charlie.publicKey, {
                    mint,
                })
            ).items;

            expect(senderAccounts.length).toBe(1);
            expect(receiverAccounts.length).toBe(1);
            expect(
                senderAccounts[0].compressedAccount.merkleTree.toBase58(),
            ).toBe(outputStateTreeInfoV2.tree.toBase58());
            expect(
                receiverAccounts[0].compressedAccount.merkleTree.toBase58(),
            ).toBe(outputStateTreeInfoV2.tree.toBase58());
        });

        it('getCompressedTokenAccountBalance should return consistent tree and queue ', async () => {
            const senderAccounts = await rpc.getCompressedTokenAccountsByOwner(
                bob.publicKey,
                { mint },
            );
            expect(
                senderAccounts.items[0].compressedAccount.merkleTree.toBase58(),
            ).toBe(outputStateTreeInfoV2.tree.toBase58());
            expect(
                senderAccounts.items[0].compressedAccount.queue?.toBase58(),
            ).toBe(outputStateTreeInfoV2.queue?.toBase58());
        });

        it('should return both compressed token accounts in different trees', async () => {
            const mintAmount = bn(1000);
            await mintTo(
                rpc,
                payer,
                mint,
                bob.publicKey,
                mintAuthority,
                mintAmount,
                outputStateTreeInfo,
            );

            const senderAccounts = await rpc.getCompressedTokenAccountsByOwner(
                bob.publicKey,
                { mint },
            );
            const previousAccount = senderAccounts.items.find(
                account =>
                    account.compressedAccount.merkleTree.toBase58() ===
                    outputStateTreeInfoV2.tree.toBase58(),
            );

            const newlyMintedAccount = senderAccounts.items.find(
                account =>
                    account.compressedAccount.merkleTree.toBase58() ===
                    outputStateTreeInfo.tree.toBase58(),
            );

            expect(previousAccount).toBeDefined();
            expect(newlyMintedAccount).toBeDefined();
            expect(newlyMintedAccount!.parsed.amount.toNumber()).toBe(
                mintAmount.toNumber(),
            );
        });
    },
);
