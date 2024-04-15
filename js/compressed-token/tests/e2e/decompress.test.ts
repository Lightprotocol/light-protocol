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
async function assertDecompress(
    rpc: Rpc,
    refRecipientAtaBalanceBefore: BN,
    refRecipientAta: PublicKey, // all
    refMint: PublicKey,
    refAmount: BN,
    refSender: PublicKey,
    refSenderCompressedTokenBalanceBefore: ParsedTokenAccount[],
) {
    const refRecipientAtaBalanceAfter =
        await rpc.getTokenAccountBalance(refRecipientAta);

    const senderCompressedTokenBalanceAfter =
        await rpc.getCompressedTokenAccountsByOwner(refSender, {
            mint: refMint,
        });

    const senderSumPost = senderCompressedTokenBalanceAfter.reduce(
        (acc, curr) => bn(acc).add(curr.parsed.amount),
        bn(0),
    );
    const senderSumPre = refSenderCompressedTokenBalanceBefore.reduce(
        (acc, curr) => bn(acc).add(curr.parsed.amount),
        bn(0),
    );

    /// recipient ata should have received the amount
    expect(
        bn(refRecipientAtaBalanceAfter.value.amount)
            .sub(refAmount)
            .eq(refRecipientAtaBalanceBefore),
    ).toBe(true);

    /// should have sent the amount
    expect(senderSumPost.eq(senderSumPre.sub(refAmount))).toBe(true);
}

const TEST_TOKEN_DECIMALS = 2;

describe('decompress', () => {
    let rpc: Rpc;
    let payer: Signer;
    let bob: Signer;

    let charlie: Signer;
    let charlieAta: PublicKey;
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

        charlieAta = await createAssociatedTokenAccount(
            rpc,
            payer,
            mint,
            charlie.publicKey,
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
    });

    it('should decompress from bob -> charlieAta', async () => {
        const rpc = createRpc();
        const recipientAtaBalanceBefore =
            await rpc.getTokenAccountBalance(charlieAta);
        const senderCompressedTokenBalanceBefore =
            await rpc.getCompressedTokenAccountsByOwner(bob.publicKey, {
                mint,
            });

        await decompress(
            rpc,
            payer,
            mint,
            bn(900),
            bob,
            charlieAta,
            merkleTree,
        );

        await assertDecompress(
            rpc,
            bn(recipientAtaBalanceBefore.value.amount),
            charlieAta,
            mint,
            bn(900),
            bob.publicKey,
            senderCompressedTokenBalanceBefore,
        );
    });
});
