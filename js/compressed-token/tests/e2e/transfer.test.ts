import { describe, it, expect, beforeAll, beforeEach } from 'vitest';
import { PublicKey, Keypair, Signer } from '@solana/web3.js';
import { BN } from '@coral-xyz/anchor';
import {
    Rpc,
    bn,
    createRpc,
    defaultTestStateTreeAccounts,
    newAccountWithLamports,
} from '@lightprotocol/stateless.js';
import { createMint, mintTo, transfer } from '../../src/actions';
import { CompressedAccountWithParsedTokenData } from '../../src/get-compressed-token-accounts';

/**
 * Assert that we created recipient and change ctokens for the sender, with all
 * amounts correctly accounted for
 */
// TODO: assert individual account amounts in balance
async function assertTransfer(
    rpc: Rpc,
    senderPreCompressedTokenAccounts: CompressedAccountWithParsedTokenData[], // all
    refMint: PublicKey,
    refAmount: BN,
    refSender: PublicKey,
    refRecipient: PublicKey,
    expectedAccountCountSenderPost?: number,
    expectedAccountCountRecipientPost?: number,
) {
    /// Transfer can merge input compressedaccounts therefore we need to pass all as ref
    const senderPostCompressedTokenAccounts =
        await rpc.getCompressedTokenAccountsByOwner(refSender, {
            mint: refMint,
        });
    /// pre = post-amount
    const sumPre = senderPreCompressedTokenAccounts.reduce(
        (acc, curr) => bn(acc).add(curr.parsed.amount),
        bn(0),
    );
    const sumPost = senderPostCompressedTokenAccounts.reduce(
        (acc, curr) => bn(acc).add(curr.parsed.amount),
        bn(0),
    );

    if (expectedAccountCountSenderPost) {
        expect(senderPostCompressedTokenAccounts.length).toBe(
            expectedAccountCountSenderPost,
        );
    }

    expect(sumPre.sub(refAmount).eq(sumPost)).toBe(true);

    const recipientCompressedTokenAccounts =
        await rpc.getCompressedTokenAccountsByOwner(refRecipient, {
            mint: refMint,
        });

    if (expectedAccountCountRecipientPost) {
        expect(recipientCompressedTokenAccounts.length).toBe(
            expectedAccountCountRecipientPost,
        );
    }

    /// recipient should have received the amount
    const recipientCompressedTokenAccount = recipientCompressedTokenAccounts[0];
    expect(recipientCompressedTokenAccount.parsed.amount.eq(refAmount)).toBe(
        true,
    );
    expect(recipientCompressedTokenAccount.parsed.delegate).toBe(null);
}

const TEST_TOKEN_DECIMALS = 2;

describe('transfer', () => {
    let rpc: Rpc;
    let payer: Signer;
    let bob: Signer;
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
    });

    beforeEach(async () => {
        bob = await newAccountWithLamports(rpc);
        charlie = await newAccountWithLamports(rpc);

        await mintTo(
            rpc,
            payer,
            mint,
            bob.publicKey,
            mintAuthority,
            bn(1000),
            [],
            merkleTree,
        );
    });

    it('should transfer from bob -> charlie', async () => {
        /// send 700 from bob -> charlie
        /// bob: 300, charlie: 700

        const rpc = createRpc();

        const bobPreCompressedTokenAccounts =
            await rpc.getCompressedTokenAccountsByOwner(bob.publicKey, {
                mint,
            });

        await transfer(
            rpc,
            payer,
            mint,
            bn(700),
            bob,
            charlie.publicKey,
            merkleTree,
        );

        await assertTransfer(
            rpc,
            bobPreCompressedTokenAccounts,
            mint,
            bn(700),
            bob.publicKey,
            charlie.publicKey,
            1,
            1,
        );

        /// send 200 from bob -> charlie
        /// bob: 100, charlie: (700+200)
        const bobPreCompressedTokenAccounts2 =
            await rpc.getCompressedTokenAccountsByOwner(bob.publicKey, {
                mint,
            });
        await transfer(
            rpc,
            payer,
            mint,
            bn(200),
            bob,
            charlie.publicKey,
            merkleTree,
        );

        await assertTransfer(
            rpc,
            bobPreCompressedTokenAccounts2,
            mint,
            bn(200),
            bob.publicKey,
            charlie.publicKey,
            1,
            2,
        );

        /// send 5 from charlie -> bob
        /// bob: (100+5), charlie: (695+200)

        const charliePreCompressedTokenAccounts3 =
            await rpc.getCompressedTokenAccountsByOwner(charlie.publicKey, {
                mint,
            });

        await transfer(
            rpc,
            payer,
            mint,
            bn(5),
            charlie,
            bob.publicKey,
            merkleTree,
        );

        await assertTransfer(
            rpc,
            charliePreCompressedTokenAccounts3,
            mint,
            bn(5),
            charlie.publicKey,
            bob.publicKey,
            2,
            2,
        );

        /// send 700 from charlie -> bob, 2 compressed account inputs
        /// bob: (100+5+700), charlie: (195)

        const charliePreCompressedTokenAccounts4 =
            await rpc.getCompressedTokenAccountsByOwner(charlie.publicKey, {
                mint,
            });
        await transfer(
            rpc,
            payer,
            mint,
            bn(700),
            charlie,
            bob.publicKey,
            merkleTree,
        );

        await assertTransfer(
            rpc,
            charliePreCompressedTokenAccounts4,
            mint,
            bn(700),
            charlie.publicKey,
            bob.publicKey,
            1,
            3,
        );

        await expect(
            transfer(
                rpc,
                payer,
                mint,
                10000,
                bob,
                charlie.publicKey,
                merkleTree,
            ),
        ).rejects.toThrow('Not enough balance for transfer');
    });
});
