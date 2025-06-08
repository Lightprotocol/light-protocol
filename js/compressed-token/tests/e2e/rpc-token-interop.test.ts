import { describe, it, assert, beforeAll } from 'vitest';
import { Keypair, PublicKey, Signer } from '@solana/web3.js';
import {
    Rpc,
    newAccountWithLamports,
    bn,
    createRpc,
    getTestRpc,
    defaultTestStateTreeAccounts,
    TreeInfo,
    selectStateTreeInfo,
} from '@lightprotocol/stateless.js';
import { WasmFactory } from '@lightprotocol/hasher.rs';
import { createMint, mintTo, transfer } from '../../src/actions';
import {
    getTokenPoolInfos,
    selectTokenPoolInfo,
    TokenPoolInfo,
} from '../../src/utils/get-token-pool-infos';

const TEST_TOKEN_DECIMALS = 2;

describe('rpc-interop token', () => {
    let rpc: Rpc;
    let testRpc: Rpc;
    let payer: Signer;
    let bob: Signer;
    let charlie: Signer;
    let mint: PublicKey;
    let mintAuthority: Keypair;
    let stateTreeInfo: TreeInfo;
    let tokenPoolInfo: TokenPoolInfo;

    beforeAll(async () => {
        const lightWasm = await WasmFactory.getInstance();
        rpc = createRpc();
        testRpc = await getTestRpc(lightWasm);
        payer = await newAccountWithLamports(rpc);
        bob = await newAccountWithLamports(rpc);
        charlie = await newAccountWithLamports(rpc);
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

        stateTreeInfo = selectStateTreeInfo(await rpc.getStateTreeInfos());
        tokenPoolInfo = selectTokenPoolInfo(await getTokenPoolInfos(rpc, mint));

        await mintTo(
            rpc,
            payer,
            mint,
            bob.publicKey,
            mintAuthority,
            bn(1000),
            stateTreeInfo,
            tokenPoolInfo,
        );

        await transfer(rpc, payer, mint, bn(700), bob, charlie.publicKey);
    });

    it('getCompressedTokenAccountsByOwner should match', async () => {
        const senderAccounts = (
            await rpc.getCompressedTokenAccountsByOwner(bob.publicKey, { mint })
        ).items;

        const senderAccountsTest = (
            await testRpc.getCompressedTokenAccountsByOwner(bob.publicKey, {
                mint,
            })
        ).items;

        assert.equal(senderAccounts.length, senderAccountsTest.length);

        senderAccounts.forEach((account, index) => {
            assert.equal(
                account.parsed.owner.toBase58(),
                senderAccountsTest[index].parsed.owner.toBase58(),
            );
            assert.isTrue(
                account.parsed.amount.eq(
                    senderAccountsTest[index].parsed.amount,
                ),
            );
        });

        const receiverAccounts = (
            await rpc.getCompressedTokenAccountsByOwner(charlie.publicKey, {
                mint,
            })
        ).items;

        const receiverAccountsTest = (
            await testRpc.getCompressedTokenAccountsByOwner(charlie.publicKey, {
                mint,
            })
        ).items;

        assert.equal(receiverAccounts.length, receiverAccountsTest.length);
        receiverAccounts.forEach((account, index) => {
            assert.equal(
                account.parsed.owner.toBase58(),
                receiverAccountsTest[index].parsed.owner.toBase58(),
            );
            assert.isTrue(
                account.parsed.amount.eq(
                    receiverAccountsTest[index].parsed.amount,
                ),
            );
        });
    });

    it('getCompressedTokenAccountBalance should match ', async () => {
        const senderAccounts = await rpc.getCompressedTokenAccountsByOwner(
            bob.publicKey,
            { mint },
        );

        const balance = await rpc.getCompressedTokenAccountBalance(
            bn(senderAccounts.items[0].compressedAccount.hash),
        );
        const balanceTest = await testRpc.getCompressedTokenAccountBalance(
            bn(senderAccounts.items[0].compressedAccount.hash),
        );
        assert.isTrue(balance.amount.eq(balanceTest.amount));
        assert.isNotNull(balance.amount);
        assert.isNotNull(balanceTest.amount);
    });

    it('getCompressedTokenBalancesByOwner should match', async () => {
        const balances = (
            await rpc.getCompressedTokenBalancesByOwner(bob.publicKey, { mint })
        ).items;
        const balancesTest = (
            await testRpc.getCompressedTokenBalancesByOwner(bob.publicKey, {
                mint,
            })
        ).items;

        assert.equal(balances.length, balancesTest.length);

        balances.forEach((balance, index) => {
            assert.isTrue(balance.balance.eq(balancesTest[index].balance));
        });

        const balancesReceiver = (
            await rpc.getCompressedTokenBalancesByOwner(charlie.publicKey, {
                mint,
            })
        ).items;
        const balancesReceiverTest = (
            await testRpc.getCompressedTokenBalancesByOwner(charlie.publicKey, {
                mint,
            })
        ).items;

        assert.equal(balancesReceiver.length, balancesReceiverTest.length);
        balancesReceiver.forEach((balance, index) => {
            assert.isTrue(
                balance.balance.eq(balancesReceiverTest[index].balance),
            );
        });
    });

    it('getCompressedTokenBalancesByOwnerV2 should match', async () => {
        const balances = (
            await rpc.getCompressedTokenBalancesByOwnerV2(bob.publicKey, {
                mint,
            })
        ).value.items;
        const balancesTest = (
            await testRpc.getCompressedTokenBalancesByOwnerV2(bob.publicKey, {
                mint,
            })
        ).value.items;

        assert.equal(balances.length, balancesTest.length);

        balances.forEach((balance, index) => {
            assert.isTrue(balance.balance.eq(balancesTest[index].balance));
        });

        const balancesReceiver = (
            await rpc.getCompressedTokenBalancesByOwnerV2(charlie.publicKey, {
                mint,
            })
        ).value.items;
        const balancesReceiverTest = (
            await testRpc.getCompressedTokenBalancesByOwnerV2(
                charlie.publicKey,
                {
                    mint,
                },
            )
        ).value.items;

        assert.equal(balancesReceiver.length, balancesReceiverTest.length);
        balancesReceiver.forEach((balance, index) => {
            assert.isTrue(
                balance.balance.eq(balancesReceiverTest[index].balance),
            );
        });
    });

    it('[test-rpc missing] getSignaturesForTokenOwner should match', async () => {
        const signatures = (
            await rpc.getCompressionSignaturesForTokenOwner(bob.publicKey)
        ).items;

        assert.equal(signatures.length, 2);

        const signaturesReceiver = (
            await rpc.getCompressionSignaturesForTokenOwner(charlie.publicKey)
        ).items;
        assert.equal(signaturesReceiver.length, 1);
    });

    it('[test-rpc missing] getTransactionWithCompressionInfo should return correct token pre and post balances', async () => {
        const signatures = (
            await rpc.getCompressionSignaturesForTokenOwner(bob.publicKey)
        ).items;

        const tx = await rpc.getTransactionWithCompressionInfo(
            // most recent
            signatures[0].signature,
        );
        assert.isTrue(
            tx!.compressionInfo.preTokenBalances![0].amount.eq(bn(1000)),
        );
        assert.isTrue(
            tx!.compressionInfo.postTokenBalances![0].amount.eq(bn(300)),
        );
        assert.isTrue(tx!.compressionInfo.postTokenBalances!.length === 2);
        assert.isTrue(tx!.compressionInfo.preTokenBalances!.length === 1);
    });

    it('[delegate unused] getCompressedTokenAccountsByDelegate should match', async () => {
        const accs = await rpc.getCompressedTokenAccountsByDelegate(
            bob.publicKey,
            { mint },
        );

        assert.equal(accs.items.length, 0);
    });

    it('[rpc] getCompressedTokenAccountsByOwner with 2 mints should return both mints', async () => {
        // additional mint
        const mint2 = (
            await createMint(
                rpc,
                payer,
                mintAuthority.publicKey,
                TEST_TOKEN_DECIMALS,
            )
        ).mint;

        const tokenPoolInfo2 = selectTokenPoolInfo(
            await getTokenPoolInfos(rpc, mint2),
        );

        await mintTo(
            rpc,
            payer,
            mint2,
            bob.publicKey,
            mintAuthority,
            bn(1000),
            stateTreeInfo,
            tokenPoolInfo2,
        );

        const senderAccounts = await rpc.getCompressedTokenAccountsByOwner(
            bob.publicKey,
        );

        // check that mint and mint2 exist in list of senderaccounts at least once
        assert.isTrue(
            senderAccounts.items.some(
                account => account.parsed.mint.toBase58() === mint.toBase58(),
            ),
        );
        assert.isTrue(
            senderAccounts.items.some(
                account => account.parsed.mint.toBase58() === mint2.toBase58(),
            ),
        );
    });

    it('getCompressedMintTokenHolders should return correct holders', async () => {
        const holders = await rpc.getCompressedMintTokenHolders(mint);

        assert.equal(holders.value.items.length, 2);

        const bobHolder = holders.value.items.find(
            holder => holder.owner.toBase58() === bob.publicKey.toBase58(),
        );
        assert.isNotNull(bobHolder);
        assert.isTrue(bobHolder!.balance.eq(bn(300)));

        const charlieHolder = holders.value.items.find(
            holder => holder.owner.toBase58() === charlie.publicKey.toBase58(),
        );
        assert.isNotNull(charlieHolder);
        assert.isTrue(charlieHolder!.balance.eq(bn(700)));
    });

    it('getCompressedMintTokenHolders should handle cursor and limit', async () => {
        // Get first holder with limit 1
        const firstPage = await rpc.getCompressedMintTokenHolders(mint, {
            limit: bn(1),
        });
        assert.equal(firstPage.value.items.length, 1);
        assert.isNotNull(firstPage.value.cursor);

        // Get second holder using cursor
        const secondPage = await rpc.getCompressedMintTokenHolders(mint, {
            cursor: firstPage.value.cursor!,
            limit: bn(1),
        });
        assert.equal(secondPage.value.items.length, 1);

        // Verify we got both holders across the pages
        const allHolders = [
            ...firstPage.value.items,
            ...secondPage.value.items,
        ];
        assert.equal(allHolders.length, 2);

        const hasCharlie = allHolders.some(
            holder => holder.owner.toBase58() === charlie.publicKey.toBase58(),
        );
        const hasBob = allHolders.some(
            holder => holder.owner.toBase58() === bob.publicKey.toBase58(),
        );

        assert.isTrue(hasCharlie && hasBob);
    });
});
