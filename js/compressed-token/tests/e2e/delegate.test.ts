import { describe, it, expect, beforeAll, beforeEach } from 'vitest';
import { PublicKey, Keypair, Signer } from '@solana/web3.js';
import BN from 'bn.js';
import {
    Rpc,
    bn,
    newAccountWithLamports,
    getTestRpc,
    StateTreeInfo,
    selectStateTreeInfo,
} from '@lightprotocol/stateless.js';
import { WasmFactory } from '@lightprotocol/hasher.rs';
import { createMint, mintTo, approve, revoke } from '../../src/actions';
import {
    getTokenPoolInfos,
    selectTokenPoolInfo,
    TokenPoolInfo,
} from '../../src/utils/get-token-pool-infos';
import { SendTransactionError } from '@solana/web3.js';

/**
 * Assert that delegation was successful
 */
async function assertDelegate(
    rpc: Rpc,
    refMint: PublicKey,
    refAmount: BN,
    refOwner: PublicKey,
    refDelegate: PublicKey,
    expectedAccountCountOwnerPost?: number,
    expectedAccountCountDelegatePost?: number,
) {
    const ownerPostCompressedTokenAccounts = (
        await rpc.getCompressedTokenAccountsByOwner(refOwner, {
            mint: refMint,
        })
    ).items;

    const delegateCompressedTokenAccounts = (
        await rpc.getCompressedTokenAccountsByDelegate(refDelegate, {
            mint: refMint,
        })
    ).items;

    if (expectedAccountCountOwnerPost) {
        expect(ownerPostCompressedTokenAccounts.length).toBe(
            expectedAccountCountOwnerPost,
        );
    }

    if (expectedAccountCountDelegatePost) {
        expect(delegateCompressedTokenAccounts.length).toBe(
            expectedAccountCountDelegatePost,
        );
    }

    // Check that delegate has the delegated amount
    const delegatedAmount = delegateCompressedTokenAccounts.reduce(
        (sum, acc) => sum.add(acc.parsed.amount),
        bn(0),
    );
    expect(delegatedAmount.eq(refAmount)).toBe(true);

    // Check that delegate is set correctly
    expect(
        delegateCompressedTokenAccounts.every(acc =>
            acc.parsed.delegate?.equals(refDelegate),
        ),
    ).toBe(true);
}

const TEST_TOKEN_DECIMALS = 2;

describe('delegate', () => {
    let rpc: Rpc;
    let payer: Signer;
    let bob: Signer;
    let mint: PublicKey;
    let mintAuthority: Keypair;
    let stateTreeInfo: StateTreeInfo;
    let tokenPoolInfo: TokenPoolInfo;

    beforeAll(async () => {
        const lightWasm = await WasmFactory.getInstance();
        rpc = await getTestRpc(lightWasm);
        payer = await newAccountWithLamports(rpc, 1e9);
        mintAuthority = Keypair.generate();
        const mintKeypair = Keypair.generate();

        stateTreeInfo = selectStateTreeInfo(await rpc.getStateTreeInfos());

        mint = (
            await createMint(
                rpc,
                payer,
                mintAuthority.publicKey,
                TEST_TOKEN_DECIMALS,
                mintKeypair,
            )
        ).mint;

        tokenPoolInfo = selectTokenPoolInfo(await getTokenPoolInfos(rpc, mint));
    });

    beforeEach(async () => {
        bob = await newAccountWithLamports(rpc, 1e9);

        // Mint twice to create two token accounts
        await mintTo(
            rpc,
            payer,
            mint,
            payer.publicKey,
            mintAuthority,
            bn(500),
            stateTreeInfo,
            tokenPoolInfo,
        );
        await mintTo(
            rpc,
            payer,
            mint,
            payer.publicKey,
            mintAuthority,
            bn(500),
            stateTreeInfo,
            tokenPoolInfo,
        );
    });

    it('should approve and revoke all tokens', async () => {
        const payerPreCompressedTokenAccounts = (
            await rpc.getCompressedTokenAccountsByOwner(payer.publicKey, {
                mint,
            })
        ).items;

        const totalAmount = payerPreCompressedTokenAccounts.reduce(
            (sum, acc) => sum.add(acc.parsed.amount),
            bn(0),
        );

        // Approve all tokens to payer
        await approve(rpc, payer, mint, totalAmount, payer, bob.publicKey);

        await assertDelegate(
            rpc,
            mint,
            totalAmount,
            payer.publicKey,
            bob.publicKey,
            1, // Merged!
            1, // Delegate should have one account
        );

        // Revoke all tokens
        const delegatedAccounts = (
            await rpc.getCompressedTokenAccountsByDelegate(bob.publicKey, {
                mint,
            })
        ).items;

        await revoke(rpc, payer, delegatedAccounts, payer);

        // Verify all tokens are back with owner
        const payerPostCompressedTokenAccounts = (
            await rpc.getCompressedTokenAccountsByOwner(payer.publicKey, {
                mint,
            })
        ).items;

        const postAmount = payerPostCompressedTokenAccounts.reduce(
            (sum, acc) => sum.add(acc.parsed.amount),
            bn(0),
        );
        expect(postAmount.eq(totalAmount)).toBe(true);

        // verify no delegate accounts
        const bobPostCompressedTokenAccounts = (
            await rpc.getCompressedTokenAccountsByDelegate(bob.publicKey, {
                mint,
            })
        ).items;
        expect(bobPostCompressedTokenAccounts.length).toBe(0);
    });

    it('should approve and revoke partial amount', async () => {
        const payerPreCompressedTokenAccounts = (
            await rpc.getCompressedTokenAccountsByOwner(payer.publicKey, {
                mint,
            })
        ).items;

        const totalAmount = payerPreCompressedTokenAccounts.reduce(
            (sum, acc) => sum.add(acc.parsed.amount),
            bn(0),
        );

        // Approve partial amount (700) to payer
        const delegateAmount = bn(700);
        await approve(rpc, payer, mint, delegateAmount, payer, bob.publicKey);

        await assertDelegate(
            rpc,
            mint,
            delegateAmount,
            payer.publicKey,
            bob.publicKey,
            payerPreCompressedTokenAccounts.length + 1,
            1, // Delegate should have one account
        );

        // Revoke delegated tokens
        const delegatedAccounts = (
            await rpc.getCompressedTokenAccountsByDelegate(bob.publicKey, {
                mint,
            })
        ).items;

        await revoke(rpc, payer, delegatedAccounts, payer);

        // Verify all tokens are back with owner
        const payerPostCompressedTokenAccounts = (
            await rpc.getCompressedTokenAccountsByOwner(payer.publicKey, {
                mint,
            })
        ).items;

        // delegate are gone
        const bobPostCompressedTokenAccountsDelegate = (
            await rpc.getCompressedTokenAccountsByDelegate(bob.publicKey, {
                mint,
            })
        ).items;

        expect(bobPostCompressedTokenAccountsDelegate.length).toBe(0);

        expect(payerPostCompressedTokenAccounts.length).toBe(
            payerPreCompressedTokenAccounts.length + 1,
        );

        const postAmount = payerPostCompressedTokenAccounts.reduce(
            (sum, acc) => sum.add(acc.parsed.amount),
            bn(0),
        );
        expect(postAmount.eq(totalAmount)).toBe(true);
    });

    it('should approve and revoke single token account', async () => {
        const payerPreCompressedTokenAccounts = (
            await rpc.getCompressedTokenAccountsByOwner(payer.publicKey, {
                mint,
            })
        ).items;

        // Approve first token account's amount (500) to payer
        const firstAccountAmount =
            payerPreCompressedTokenAccounts[0].parsed.amount;

        await approve(
            rpc,
            payer,
            mint,
            firstAccountAmount,
            payer,
            bob.publicKey,
        );

        await assertDelegate(
            rpc,
            mint,
            firstAccountAmount,
            payer.publicKey,
            bob.publicKey,
            payerPreCompressedTokenAccounts.length,
            1, // Delegate should have one account
        );

        // Revoke delegated tokens
        const delegatedAccounts = (
            await rpc.getCompressedTokenAccountsByDelegate(bob.publicKey, {
                mint,
            })
        ).items;

        await revoke(rpc, payer, delegatedAccounts, payer);

        // Verify all tokens are back with owner
        const payerPostCompressedTokenAccounts = (
            await rpc.getCompressedTokenAccountsByOwner(payer.publicKey, {
                mint,
            })
        ).items;

        // and that delegatge no more
        const bobPostCompressedTokenAccounts = (
            await rpc.getCompressedTokenAccountsByDelegate(bob.publicKey, {
                mint,
            })
        ).items;

        expect(bobPostCompressedTokenAccounts.length).toBe(0);

        const postAmount = payerPostCompressedTokenAccounts.reduce(
            (sum, acc) => sum.add(acc.parsed.amount),
            bn(0),
        );
        const totalAmount = payerPreCompressedTokenAccounts.reduce(
            (sum, acc) => sum.add(acc.parsed.amount),
            bn(0),
        );
        expect(postAmount.eq(totalAmount)).toBe(true);
    });

    it('should approve and revoke when payer is not owner', async () => {
        const charlie = await newAccountWithLamports(rpc, 1e9);
        const payerPreCompressedTokenAccounts = (
            await rpc.getCompressedTokenAccountsByOwner(payer.publicKey, {
                mint,
            })
        ).items;

        const totalAmount = payerPreCompressedTokenAccounts.reduce(
            (sum, acc) => sum.add(acc.parsed.amount),
            bn(0),
        );

        // Approve all tokens to bob using payer as fee payer
        await approve(rpc, bob, mint, totalAmount, payer, charlie.publicKey);

        await assertDelegate(
            rpc,
            mint,
            totalAmount,
            payer.publicKey,
            charlie.publicKey,
            1, // Merged!
            1, // Delegate should have one account
        );

        // Revoke all tokens using payer as fee payer
        const delegatedAccounts = (
            await rpc.getCompressedTokenAccountsByDelegate(charlie.publicKey, {
                mint,
            })
        ).items;

        await revoke(rpc, bob, delegatedAccounts, payer);

        // Verify all tokens are back with owner
        const payerPostCompressedTokenAccounts = (
            await rpc.getCompressedTokenAccountsByOwner(payer.publicKey, {
                mint,
            })
        ).items;

        const postAmount = payerPostCompressedTokenAccounts.reduce(
            (sum, acc) => sum.add(acc.parsed.amount),
            bn(0),
        );
        expect(postAmount.eq(totalAmount)).toBe(true);
    });

    it('should fail when non-owner tries to approve or revoke', async () => {
        const payerPreCompressedTokenAccounts = (
            await rpc.getCompressedTokenAccountsByOwner(payer.publicKey, {
                mint,
            })
        ).items;

        const totalAmount = payerPreCompressedTokenAccounts.reduce(
            (sum, acc) => sum.add(acc.parsed.amount),
            bn(0),
        );

        // Try to approve using non-owner (bob)
        try {
            await approve(rpc, bob, mint, totalAmount, payer, bob.publicKey);
            expect.fail('Should have failed when non-owner tries to approve');
        } catch (e) {
            if (e instanceof SendTransactionError) {
                console.error(
                    'Transaction failed with logs:',
                    await e.getLogs(rpc),
                );
            }
            expect(e).toBeDefined();
        }

        // Try to revoke using non-owner (bob)
        const delegatedAccounts = (
            await rpc.getCompressedTokenAccountsByDelegate(bob.publicKey, {
                mint,
            })
        ).items;

        try {
            await revoke(rpc, bob, delegatedAccounts, payer);
            expect.fail('Should have failed when non-owner tries to revoke');
        } catch (e) {
            if (e instanceof SendTransactionError) {
                console.error(
                    'Transaction failed with logs:',
                    await e.getLogs(rpc),
                );
            }
            expect(e).toBeDefined();
        }
    });
});
