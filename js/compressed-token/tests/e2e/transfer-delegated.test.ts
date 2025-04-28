import { describe, it, expect, beforeAll } from 'vitest';
import { PublicKey, Keypair, Signer } from '@solana/web3.js';
import BN from 'bn.js';
import {
    Rpc,
    bn,
    newAccountWithLamports,
    getTestRpc,
    StateTreeInfo,
    selectStateTreeInfo,
    ParsedTokenAccount,
} from '@lightprotocol/stateless.js';
import { WasmFactory } from '@lightprotocol/hasher.rs';
import {
    createMint,
    mintTo,
    approve,
    transferDelegated,
} from '../../src/actions';
import {
    getTokenPoolInfos,
    selectTokenPoolInfo,
    TokenPoolInfo,
} from '../../src/utils/get-token-pool-infos';

const assertPostTransfer = async (
    rpc: Rpc,
    refMint: PublicKey,
    refAmount: BN,
    refOwner: PublicKey,
    refDelegate: PublicKey,
    refRecipient: PublicKey,
    preOwnerAccounts: ParsedTokenAccount[],
    preDelegateAccounts: ParsedTokenAccount[],
    preRecipientAccounts: ParsedTokenAccount[],
    newOwnerCount: number,
    newDelegateCount: number,
    newRecipientCount: number,
) => {
    const getAmount = async (type: 'delegate' | 'owner', pubkey: PublicKey) => {
        const accounts =
            type === 'delegate'
                ? await rpc.getCompressedTokenAccountsByDelegate(pubkey, {
                      mint: refMint,
                  })
                : await rpc.getCompressedTokenAccountsByOwner(pubkey, {
                      mint: refMint,
                  });
        return accounts.items.reduce(
            (acc, account) => acc.add(account.parsed.amount),
            bn(0),
        );
    };

    const postTransferDelegateAmount = await getAmount('delegate', refDelegate);
    const postTransferOwnerAmount = await getAmount('owner', refOwner);
    const postTransferRecipientAmount = await getAmount('owner', refRecipient);

    const logError = (
        label: string,
        expected: BN | number,
        actual: BN | number,
    ) => {
        console.error(`${label}:`);
        console.error(`  Expected: ${expected.toString()}`);
        console.error(`  Actual:   ${actual.toString()}`);
    };

    const totalAmount = postTransferDelegateAmount
        .add(postTransferOwnerAmount)
        .add(postTransferRecipientAmount);
    if (!totalAmount.eq(refAmount)) {
        logError('Total amount', refAmount, totalAmount);
    }
    expect(totalAmount.eq(refAmount)).toBe(true);

    const preDelegateTotal = preDelegateAccounts.reduce(
        (acc, account) => acc.add(account.parsed.amount),
        bn(0),
    );
    if (!preDelegateTotal.eq(postTransferDelegateAmount)) {
        logError(
            'Delegate amount',
            preDelegateTotal,
            postTransferDelegateAmount,
        );
    }
    expect(preDelegateTotal.eq(postTransferDelegateAmount.add(refAmount))).toBe(
        true,
    );

    // pre owner amount - ref amount = post owner amount
    const preOwnerTotal = preOwnerAccounts.reduce(
        (acc, account) => acc.add(account.parsed.amount),
        bn(0),
    );
    const expectedOwnerAmount = preOwnerTotal.sub(refAmount);
    if (!expectedOwnerAmount.eq(postTransferOwnerAmount)) {
        logError('Owner amount', expectedOwnerAmount, postTransferOwnerAmount);
    }
    expect(expectedOwnerAmount.eq(postTransferOwnerAmount)).toBe(true);

    const preRecipientTotal = preRecipientAccounts.reduce(
        (acc, account) => acc.add(account.parsed.amount),
        bn(0),
    );
    const expectedRecipientAmount = preRecipientTotal.add(refAmount);
    if (!expectedRecipientAmount.eq(postTransferRecipientAmount)) {
        logError(
            'Recipient amount',
            expectedRecipientAmount,
            postTransferRecipientAmount,
        );
    }
    expect(expectedRecipientAmount.eq(postTransferRecipientAmount)).toBe(true);

    const postDelegateAccounts = (
        await rpc.getCompressedTokenAccountsByDelegate(refDelegate, {
            mint: refMint,
        })
    ).items;
    if (postDelegateAccounts.length !== newDelegateCount) {
        logError(
            'Delegate accounts count',
            newDelegateCount,
            postDelegateAccounts.length,
        );
    }
    expect(postDelegateAccounts.length).toBe(newDelegateCount);

    const postOwnerAccounts = (
        await rpc.getCompressedTokenAccountsByOwner(refOwner, {
            mint: refMint,
        })
    ).items;
    if (postOwnerAccounts.length !== newOwnerCount) {
        logError(
            'Owner accounts count',
            newOwnerCount,
            postOwnerAccounts.length,
        );
    }
    expect(postOwnerAccounts.length).toBe(newOwnerCount);

    const postRecipientAccounts = (
        await rpc.getCompressedTokenAccountsByOwner(refRecipient, {
            mint: refMint,
        })
    ).items;
    if (postRecipientAccounts.length !== newRecipientCount) {
        logError(
            'Recipient accounts count',
            newRecipientCount,
            postRecipientAccounts.length,
        );
    }
    expect(postRecipientAccounts.length).toBe(newRecipientCount);
};

const TEST_TOKEN_DECIMALS = 2;

describe('transferDelegated', () => {
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
        bob = await newAccountWithLamports(rpc, 1e9);
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

        await mintTo(
            rpc,
            payer,
            mint,
            payer.publicKey,
            mintAuthority,
            bn(1000),
            stateTreeInfo,
            tokenPoolInfo,
        );

        await approve(rpc, payer, mint, 1000, payer, bob.publicKey);
    });

    it('should transfer one delegated account', async () => {
        const charlie = await newAccountWithLamports(rpc, 1e9);
        const payerPreCompressedTokenAccounts = (
            await rpc.getCompressedTokenAccountsByOwner(payer.publicKey, {
                mint,
            })
        ).items;

        const preDelegateAccounts = (
            await rpc.getCompressedTokenAccountsByDelegate(bob.publicKey, {
                mint,
            })
        ).items;

        await transferDelegated(rpc, bob, mint, 1000, bob, charlie.publicKey);

        await assertPostTransfer(
            rpc,
            mint,
            bn(1000),
            payer.publicKey,
            bob.publicKey,
            charlie.publicKey,
            payerPreCompressedTokenAccounts,
            preDelegateAccounts,
            [], // charlie
            0,
            0,
            1,
        );
    });
});
