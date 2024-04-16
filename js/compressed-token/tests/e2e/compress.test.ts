import { describe, it, expect, beforeAll } from 'vitest';
import { PublicKey, Keypair, Signer } from '@solana/web3.js';
import { BN } from '@coral-xyz/anchor';
import {
    ParsedTokenAccount,
    Rpc,
    bn,
    createRpc,
    defaultTestStateTreeAccounts,
    newAccountWithLamports,
} from '@lightprotocol/stateless.js';
import { compress, createMint, decompress, mintTo } from '../../src/actions';
import { createAssociatedTokenAccount } from '@solana/spl-token';

/**
 * Assert that we created recipient and change ctokens for the sender, with all
 * amounts correctly accounted for
 */
async function assertCompress(
    rpc: Rpc,
    refSenderAtaBalanceBefore: BN,
    refSenderAta: PublicKey, // all
    refMint: PublicKey,
    refAmount: BN,
    refRecipient: PublicKey,
    refRecipientCompressedTokenBalanceBefore: ParsedTokenAccount[],
) {
    const refSenderAtaBalanceAfter =
        await rpc.getTokenAccountBalance(refSenderAta);

    const recipientCompressedTokenBalanceAfter =
        await rpc.getCompressedTokenAccountsByOwner(refRecipient, {
            mint: refMint,
        });

    const recipientSumPost = recipientCompressedTokenBalanceAfter.reduce(
        (acc, curr) => bn(acc).add(curr.parsed.amount),
        bn(0),
    );
    const recipientSumPre = refRecipientCompressedTokenBalanceBefore.reduce(
        (acc, curr) => bn(acc).add(curr.parsed.amount),
        bn(0),
    );

    expect(
        refSenderAtaBalanceBefore
            .sub(refAmount)
            .eq(bn(refSenderAtaBalanceAfter.value.amount)),
    ).toBe(true);

    /// recipient should have received the amount
    expect(recipientSumPost.eq(refAmount.add(recipientSumPre))).toBe(true);
}

const TEST_TOKEN_DECIMALS = 2;

describe('compress', () => {
    let rpc: Rpc;
    let payer: Signer;
    let bob: Signer;
    let bobAta: PublicKey;
    let charlie: Signer;
    let mint: PublicKey;
    let mintAuthority: Keypair;
    const { merkleTree } = defaultTestStateTreeAccounts();

    beforeAll(async () => {
        rpc = createRpc();
        payer = await newAccountWithLamports(rpc);
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

        bob = await newAccountWithLamports(rpc);
        charlie = await newAccountWithLamports(rpc);

        bobAta = await createAssociatedTokenAccount(
            rpc,
            payer,
            mint,
            bob.publicKey,
        );

        await mintTo(
            rpc,
            payer,
            mint,
            bob.publicKey,
            mintAuthority,
            bn(1000),
            merkleTree,
        );

        await decompress(rpc, payer, mint, bn(900), bob, bobAta, merkleTree);


    });

    it('should compress from bobAta -> charlie', async () => {
        const rpc = createRpc();
        const senderAtaBalanceBefore = await rpc.getTokenAccountBalance(bobAta);
        const recipientCompressedTokenBalanceBefore =
            await rpc.getCompressedTokenAccountsByOwner(charlie.publicKey, {
                mint,
            });

        await compress(
            rpc,
            payer,
            mint,
            bn(700),
            bob,
            bobAta,
            charlie.publicKey,
            merkleTree,
        );
        await assertCompress(
            rpc,
            bn(senderAtaBalanceBefore.value.amount),
            bobAta,
            mint,
            bn(700),
            charlie.publicKey,
            recipientCompressedTokenBalanceBefore,
        );
    });
});
