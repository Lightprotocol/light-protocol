import { describe, it, expect, beforeAll, beforeEach, assert } from 'vitest';
import {
    PublicKey,
    Keypair,
    Signer,
    ComputeBudgetProgram,
} from '@solana/web3.js';
import BN from 'bn.js';
import {
    ParsedTokenAccount,
    Rpc,
    bn,
    defaultTestStateTreeAccounts,
    newAccountWithLamports,
    getTestRpc,
    TestRpc,
} from '@lightprotocol/stateless.js';
import { WasmFactory } from '@lightprotocol/hasher.rs';

import {
    createMint,
    createTokenProgramLookupTable,
    mintTo,
    transfer,
} from '../../src/actions';
import { TOKEN_2022_PROGRAM_ID } from '@solana/spl-token';
import { CompressedTokenProgram } from '../../src';

/**
 * Assert that we created recipient and change-account for the sender, with all
 * amounts correctly accounted for
 */
async function assertTransfer(
    rpc: Rpc,
    senderPreCompressedTokenAccounts: ParsedTokenAccount[],
    refMint: PublicKey,
    refAmount: BN,
    refSender: PublicKey,
    refRecipient: PublicKey,
    expectedAccountCountSenderPost?: number,
    expectedAccountCountRecipientPost?: number,
) {
    /// Transfer can merge input compressed accounts therefore we need to pass all as ref
    const senderPostCompressedTokenAccounts = (
        await rpc.getCompressedTokenAccountsByOwner(refSender, {
            mint: refMint,
        })
    ).items;
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

    const recipientCompressedTokenAccounts = (
        await rpc.getCompressedTokenAccountsByOwner(refRecipient, {
            mint: refMint,
        })
    ).items;

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
    let rpc: TestRpc;
    let payer: Signer;
    let bob: Signer;
    let charlie: Signer;
    let mint: PublicKey;
    let mintAuthority: Keypair;
    const { merkleTree } = defaultTestStateTreeAccounts();

    beforeAll(async () => {
        const lightWasm = await WasmFactory.getInstance();
        rpc = await getTestRpc(lightWasm);
        payer = await newAccountWithLamports(rpc, 1e9);
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
    });

    beforeEach(async () => {
        bob = await newAccountWithLamports(rpc, 1e9);
        charlie = await newAccountWithLamports(rpc, 1e9);

        await mintTo(
            rpc,
            payer,
            mint,
            bob.publicKey,
            mintAuthority,
            bn(1000),
            defaultTestStateTreeAccounts().merkleTree,
        );
    });

    it('should transfer from bob -> charlie', async () => {
        /// send 700 from bob -> charlie
        /// bob: 300, charlie: 700
        const bobPreCompressedTokenAccounts = (
            await rpc.getCompressedTokenAccountsByOwner(bob.publicKey, {
                mint,
            })
        ).items;

        const txId = await transfer(
            rpc,
            payer,
            mint,
            bn(700),
            bob,
            charlie.publicKey,
            merkleTree,
        );
        console.log('txId1', txId);

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
        const txId2 = await transfer(
            rpc,
            payer,
            mint,
            bn(200),
            bob,
            charlie.publicKey,
            merkleTree,
        );
        console.log('txId2', txId2);
        await assertTransfer(
            rpc,
            bobPreCompressedTokenAccounts2.items,
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

        const txId3 = await transfer(
            rpc,
            payer,
            mint,
            bn(5),
            charlie,
            bob.publicKey,
            merkleTree,
        );
        console.log('txId3', txId3);
        await assertTransfer(
            rpc,
            charliePreCompressedTokenAccounts3.items,
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
        const txId4 = await transfer(
            rpc,
            payer,
            mint,
            bn(700),
            charlie,
            bob.publicKey,
        );
        console.log('txId4', txId4);
        await assertTransfer(
            rpc,
            charliePreCompressedTokenAccounts4.items,
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

    it('should transfer token 2022 from bob -> charlie', async () => {
        const mintKeypair = Keypair.generate();

        mint = (
            await createMint(
                rpc,
                payer,
                mintAuthority.publicKey,
                TEST_TOKEN_DECIMALS,
                mintKeypair,
                undefined,
                true,
            )
        ).mint;
        const mintAccountInfo = await rpc.getAccountInfo(mint);
        assert.equal(
            mintAccountInfo!.owner.toBase58(),
            TOKEN_2022_PROGRAM_ID.toBase58(),
        );
        await mintTo(
            rpc,
            payer,
            mint,
            bob.publicKey,
            mintAuthority,
            bn(1000),
            defaultTestStateTreeAccounts().merkleTree,
        );

        /// send 700 from bob -> charlie
        /// bob: 300, charlie: 700

        const bobPreCompressedTokenAccounts = (
            await rpc.getCompressedTokenAccountsByOwner(bob.publicKey, {
                mint,
            })
        ).items;

        await transfer(
            rpc,
            payer,
            mint,
            bn(700),
            bob,
            charlie.publicKey,
            defaultTestStateTreeAccounts().merkleTree,
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
    });
});
