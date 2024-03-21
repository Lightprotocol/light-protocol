import { describe, it, expect, beforeAll, beforeEach } from 'vitest';
import { Connection, PublicKey, Keypair, Signer } from '@solana/web3.js';
import { BN } from '@coral-xyz/anchor';
import {
    bn,
    defaultTestStateTreeAccounts,
    getConnection,
    newAccountWithLamports,
} from '@lightprotocol/stateless.js';
import { createMint, mintTo, transfer } from '../../src/actions';
import {
    UtxoWithParsedTokenTlvData,
    getCompressedTokenAccountsFromMockRpc,
} from '../../src/token-serde';

/**
 * Assert that we created recipient and change ctokens for the sender, with all
 * amounts correctly accounted for
 */
// TODO: assert individual account amounts in balance
async function assertTransfer(
    connection: Connection,
    senderPreCompressedTokenAccounts: UtxoWithParsedTokenTlvData[], // all
    refMint: PublicKey,
    refAmount: BN,
    refSender: PublicKey,
    refRecipient: PublicKey,
    expectedAccountCountSenderPost?: number,
    expectedAccountCountRecipientPost?: number,
    // TODO: add ...refValues
) {
    /// Transfer can merge input utxos therefore we need to pass all as ref
    const senderPostCompressedTokenAccounts =
        await getCompressedTokenAccountsFromMockRpc(
            connection,
            refSender,
            refMint,
        );

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
        await getCompressedTokenAccountsFromMockRpc(
            connection,
            refRecipient,
            refMint,
        );

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
    let connection: Connection;
    let payer: Signer;
    let bob: Signer;
    let charlie: Signer;
    let mint: PublicKey;
    let mintAuthority: Keypair;
    const { merkleTree } = defaultTestStateTreeAccounts();

    beforeAll(async () => {
        connection = getConnection();
        payer = await newAccountWithLamports(connection);
        mintAuthority = Keypair.generate();
        const mintKeypair = Keypair.generate();

        mint = (
            await createMint(
                connection,
                payer,
                mintAuthority,
                TEST_TOKEN_DECIMALS,
                mintKeypair,
            )
        ).mint;
    });

    beforeEach(async () => {
        bob = await newAccountWithLamports(connection);
        charlie = await newAccountWithLamports(connection);

        await mintTo(
            connection,
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
        const bobPreCompressedTokenAccounts =
            await getCompressedTokenAccountsFromMockRpc(
                connection,
                bob.publicKey,
                mint,
            );

        await transfer(
            connection,
            payer,
            mint,
            bn(700),
            bob,
            charlie.publicKey,
            merkleTree,
        );

        await assertTransfer(
            connection,
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
            await getCompressedTokenAccountsFromMockRpc(
                connection,
                bob.publicKey,
                mint,
            );
        await transfer(
            connection,
            payer,
            mint,
            bn(200),
            bob,
            charlie.publicKey,
            merkleTree,
        );

        await assertTransfer(
            connection,
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
            await getCompressedTokenAccountsFromMockRpc(
                connection,
                charlie.publicKey,
                mint,
            );
        await transfer(
            connection,
            payer,
            mint,
            bn(5),
            charlie,
            bob.publicKey,
            merkleTree,
        );

        await assertTransfer(
            connection,
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
            await getCompressedTokenAccountsFromMockRpc(
                connection,
                charlie.publicKey,
                mint,
            );

        await transfer(
            connection,
            payer,
            mint,
            bn(700),
            charlie,
            bob.publicKey,
            merkleTree,
        );

        await assertTransfer(
            connection,
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
                connection,
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
