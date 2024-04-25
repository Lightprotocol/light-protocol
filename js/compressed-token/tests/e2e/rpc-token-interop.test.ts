import { describe, it, assert, beforeAll } from 'vitest';
import { Keypair, PublicKey, Signer } from '@solana/web3.js';

import {
    Rpc,
    newAccountWithLamports,
    getTestRpc,
    TestRpc,
    bn,
    defaultTestStateTreeAccounts,
    createRpc,
} from '@lightprotocol/stateless.js';

import { createMint, mintTo, transfer } from '../../src/actions';

const TEST_TOKEN_DECIMALS = 2;

describe('rpc-interop token', () => {
    let rpc: Rpc;
    let testRpc: TestRpc;
    let payer: Signer;
    let bob: Signer;
    let charlie: Signer;
    let mint: PublicKey;
    let mintAuthority: Keypair;
    const { merkleTree } = defaultTestStateTreeAccounts();

    beforeAll(async () => {
        rpc = createRpc();
        testRpc = await getTestRpc();
        payer = await newAccountWithLamports(rpc, 1e9, 256);
        mintAuthority = Keypair.generate();
        const mintKeypair = Keypair.generate();

        mint = (
            await createMint(
                rpc,
                payer,
                mintAuthority,
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
            merkleTree,
        );

        await transfer(
            rpc,
            payer,
            mint,
            bn(700),
            bob,
            charlie.publicKey,
            merkleTree,
        );
    });

    it('getCompressedTokenAccountsByOwner should match', async () => {
        const senderAccounts = await rpc.getCompressedTokenAccountsByOwner(
            bob.publicKey,
            { mint },
        );

        const senderAccountsTest =
            await testRpc.getCompressedTokenAccountsByOwner(bob.publicKey, {
                mint,
            });

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

        const receiverAccounts = await rpc.getCompressedTokenAccountsByOwner(
            charlie.publicKey,
            { mint },
        );
        const receiverAccountsTest =
            await testRpc.getCompressedTokenAccountsByOwner(charlie.publicKey, {
                mint,
            });

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
            bn(senderAccounts[0].compressedAccount.hash),
        );
        const balanceTest = await testRpc.getCompressedTokenAccountBalance(
            bn(senderAccounts[0].compressedAccount.hash),
        );
        assert.isTrue(balance.amount.eq(balanceTest.amount));
        assert.isNotNull(balance.amount);
        assert.isNotNull(balanceTest.amount);
    });

    it('getCompressedTokenBalancesByOwner should match', async () => {
        const balances = await rpc.getCompressedTokenBalancesByOwner(
            bob.publicKey,
            { mint },
        );
        const balancesTest = await testRpc.getCompressedTokenBalancesByOwner(
            bob.publicKey,
            { mint },
        );

        assert.equal(balances.length, balancesTest.length);

        balances.forEach((balance, index) => {
            assert.isTrue(balance.balance.eq(balancesTest[index].balance));
        });

        const balancesReceiver = await rpc.getCompressedTokenBalancesByOwner(
            charlie.publicKey,
            { mint },
        );
        const balancesReceiverTest =
            await testRpc.getCompressedTokenBalancesByOwner(charlie.publicKey, {
                mint,
            });

        assert.equal(balancesReceiver.length, balancesReceiverTest.length);
        balancesReceiver.forEach((balance, index) => {
            assert.isTrue(
                balance.balance.eq(balancesReceiverTest[index].balance),
            );
        });
    });

    it('[test-rpc missing] getSignaturesForTokenOwner should match', async () => {
        const signatures = await rpc.getSignaturesForTokenOwner(bob.publicKey);

        assert.equal(signatures.length, 2);

        const signaturesReceiver = await rpc.getSignaturesForTokenOwner(
            charlie.publicKey,
        );

        assert.equal(signaturesReceiver.length, 1);
    });

    it('[delegate unused] getCompressedTokenAccountsByDelegate should match', async () => {
        const accs = await rpc.getCompressedTokenAccountsByDelegate(
            bob.publicKey,
            { mint },
        );

        assert.equal(accs.length, 0);
    });
});
