import { describe, it, expect, beforeAll, beforeEach } from 'vitest';
import { PublicKey, Keypair, Signer } from '@solana/web3.js';
import BN from 'bn.js';
import {
    Rpc,
    bn,
    newAccountWithLamports,
    getTestRpc,
    TreeInfo,
    selectStateTreeInfo,
    ParsedTokenAccount,
} from '@lightprotocol/stateless.js';
import { WasmFactory } from '@lightprotocol/hasher.rs';
import {
    createMint,
    mintTo,
    approve,
    revoke,
    transfer,
    transferDelegated,
} from '../../src/actions';
import {
    getTokenPoolInfos,
    selectTokenPoolInfo,
    TokenPoolInfo,
} from '../../src/utils/get-token-pool-infos';

/**
 * Verifies token delegation by checking pre and post account counts and balances.
 */
async function assertDelegate(
    rpc: Rpc,
    refMint: PublicKey,
    refAmount: BN,
    refOwner: PublicKey,
    refDelegate: PublicKey,
    preOwnerAccounts: ParsedTokenAccount[],
    preDelegateAccounts: ParsedTokenAccount[],
    newOwnerCount: number,
    newDelegateCount: number,
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

    expect(ownerPostCompressedTokenAccounts.length).toBe(newOwnerCount);
    expect(delegateCompressedTokenAccounts.length).toBe(newDelegateCount);

    // Calculate pre and post balances
    const preOwnerBalance = preOwnerAccounts.reduce(
        (sum, acc) => sum.add(acc.parsed.amount),
        bn(0),
    );
    const postOwnerBalance = ownerPostCompressedTokenAccounts.reduce(
        (sum, acc) => sum.add(acc.parsed.amount),
        bn(0),
    );
    const preDelegateBalance = preDelegateAccounts.reduce(
        (sum, acc) => sum.add(acc.parsed.amount),
        bn(0),
    );
    const postDelegateBalance = delegateCompressedTokenAccounts.reduce(
        (sum, acc) => sum.add(acc.parsed.amount),
        bn(0),
    );

    // Checks
    const ownerBalanceCheck = preOwnerBalance.eq(postOwnerBalance);
    if (!ownerBalanceCheck) {
        console.log('Owner balance check failed:');
        console.log('preOwnerBalance:', preOwnerBalance.toString());
        console.log('refAmount:', refAmount.toString());
        console.log('postOwnerBalance:', postOwnerBalance.toString());
    }
    expect(ownerBalanceCheck).toBe(true);

    const delegateBalanceCheck = preDelegateBalance
        .add(refAmount)
        .eq(postDelegateBalance);
    if (!delegateBalanceCheck) {
        console.log('Delegate balance check failed:');
        console.log('preDelegateBalance:', preDelegateBalance.toString());
        console.log('refAmount:', refAmount.toString());
        console.log('postDelegateBalance:', postDelegateBalance.toString());
    }
    expect(delegateBalanceCheck).toBe(true);

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
    let stateTreeInfo: TreeInfo;
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

        const preDelegateAccounts = (
            await rpc.getCompressedTokenAccountsByDelegate(bob.publicKey, {
                mint,
            })
        ).items;
        await approve(rpc, payer, mint, totalAmount, payer, bob.publicKey);

        await assertDelegate(
            rpc,
            mint,
            totalAmount,
            payer.publicKey,
            bob.publicKey,
            payerPreCompressedTokenAccounts,
            preDelegateAccounts,
            1,
            1,
        );

        const delegatedAccounts = (
            await rpc.getCompressedTokenAccountsByDelegate(bob.publicKey, {
                mint,
            })
        ).items;

        await revoke(rpc, payer, delegatedAccounts, payer);

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

        const preDelegateAccounts = (
            await rpc.getCompressedTokenAccountsByDelegate(bob.publicKey, {
                mint,
            })
        ).items;

        const delegateAmount = bn(700);
        await approve(rpc, payer, mint, delegateAmount, payer, bob.publicKey);

        await assertDelegate(
            rpc,
            mint,
            delegateAmount,
            payer.publicKey,
            bob.publicKey,
            payerPreCompressedTokenAccounts,
            preDelegateAccounts,
            payerPreCompressedTokenAccounts.length + 1,
            1,
        );

        const delegatedAccounts = (
            await rpc.getCompressedTokenAccountsByDelegate(bob.publicKey, {
                mint,
            })
        ).items;

        await revoke(rpc, payer, delegatedAccounts, payer);

        const payerPostCompressedTokenAccounts = (
            await rpc.getCompressedTokenAccountsByOwner(payer.publicKey, {
                mint,
            })
        ).items;

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

        const firstAccountAmount =
            payerPreCompressedTokenAccounts[0].parsed.amount;

        const preDelegateAccounts = (
            await rpc.getCompressedTokenAccountsByDelegate(bob.publicKey, {
                mint,
            })
        ).items;

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
            payerPreCompressedTokenAccounts,
            preDelegateAccounts,
            payerPreCompressedTokenAccounts.length,
            1,
        );

        const delegatedAccounts = (
            await rpc.getCompressedTokenAccountsByDelegate(bob.publicKey, {
                mint,
            })
        ).items;

        await revoke(rpc, payer, delegatedAccounts, payer);

        const payerPostCompressedTokenAccounts = (
            await rpc.getCompressedTokenAccountsByOwner(payer.publicKey, {
                mint,
            })
        ).items;

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
        const owner = await newAccountWithLamports(rpc, 1e9);
        await mintTo(
            rpc,
            payer,
            mint,
            owner.publicKey,
            mintAuthority,
            bn(500),
            stateTreeInfo,
            tokenPoolInfo,
        );
        const charlie = await newAccountWithLamports(rpc, 1e9);
        const payerPreCompressedTokenAccounts = (
            await rpc.getCompressedTokenAccountsByOwner(owner.publicKey, {
                mint,
            })
        ).items;

        const totalAmount = payerPreCompressedTokenAccounts.reduce(
            (sum, acc) => sum.add(acc.parsed.amount),
            bn(0),
        );

        const preDelegateAccounts = (
            await rpc.getCompressedTokenAccountsByDelegate(charlie.publicKey, {
                mint,
            })
        ).items;

        await approve(rpc, bob, mint, totalAmount, owner, charlie.publicKey);

        await assertDelegate(
            rpc,
            mint,
            totalAmount,
            owner.publicKey,
            charlie.publicKey,
            payerPreCompressedTokenAccounts,
            preDelegateAccounts,
            1,
            1,
        );

        const delegatedAccounts = (
            await rpc.getCompressedTokenAccountsByDelegate(charlie.publicKey, {
                mint,
            })
        ).items;

        await revoke(rpc, bob, delegatedAccounts, owner);

        const ownerPostCompressedTokenAccounts = (
            await rpc.getCompressedTokenAccountsByOwner(owner.publicKey, {
                mint,
            })
        ).items;

        const postAmount = ownerPostCompressedTokenAccounts.reduce(
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

        await expect(
            approve(rpc, bob, mint, totalAmount, payer, bob.publicKey),
        ).rejects.toThrowError();

        const delegatedAccounts = (
            await rpc.getCompressedTokenAccountsByDelegate(bob.publicKey, {
                mint,
            })
        ).items;

        await expect(
            revoke(rpc, bob, delegatedAccounts, payer),
        ).rejects.toThrowError();
    });
});
