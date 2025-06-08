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
    newAccountWithLamports,
    getTestRpc,
    TestRpc,
    dedupeSigner,
    buildAndSignTx,
    sendAndConfirmTx,
    TreeInfo,
    selectStateTreeInfo,
} from '@lightprotocol/stateless.js';
import { WasmFactory } from '@lightprotocol/hasher.rs';
import { createMint, mintTo, transfer } from '../../src/actions';
import { TOKEN_2022_PROGRAM_ID } from '@solana/spl-token';
import { CompressedTokenProgram } from '../../src/program';
import { selectMinCompressedTokenAccountsForTransfer } from '../../src/utils/select-input-accounts';

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

    expect(
        recipientCompressedTokenAccounts.some(acc =>
            acc.parsed.amount.eq(refAmount),
        ),
    ).toBe(true);
    expect(
        recipientCompressedTokenAccounts.some(acc => acc.parsed.delegate),
    ).toBe(false);
}

const TEST_TOKEN_DECIMALS = 2;

describe('transfer', () => {
    let rpc: TestRpc | Rpc;
    let payer: Signer;
    let bob: Signer;
    let charlie: Signer;
    let mint: PublicKey;
    let mintAuthority: Keypair;

    let stateTreeInfo: TreeInfo;

    beforeAll(async () => {
        const lightWasm = await WasmFactory.getInstance();
        rpc = await getTestRpc(lightWasm);
        // rpc = createRpc();
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
            stateTreeInfo,
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

        const txid = await transfer(
            rpc,
            payer,
            mint,
            bn(700),
            bob,
            charlie.publicKey,
        );
        console.log('txid transfer ', txid);
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
        const txid2 = await transfer(
            rpc,
            payer,
            mint,
            bn(200),
            bob,
            charlie.publicKey,
        );
        console.log('txid transfer 2 ', txid2);
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

        await transfer(rpc, payer, mint, bn(5), charlie, bob.publicKey);

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

        await transfer(rpc, payer, mint, bn(700), charlie, bob.publicKey);

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
            transfer(rpc, payer, mint, 10000, bob, charlie.publicKey),
        ).rejects.toThrow('Insufficient balance for transfer');
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
            stateTreeInfo,
        );

        /// send 700 from bob -> charlie
        /// bob: 300, charlie: 700

        const bobPreCompressedTokenAccounts = (
            await rpc.getCompressedTokenAccountsByOwner(bob.publicKey, {
                mint,
            })
        ).items;

        await transfer(rpc, payer, mint, bn(700), bob, charlie.publicKey);

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

describe('e2e transfer with multiple accounts', () => {
    let rpc: Rpc;
    let payer: Keypair | Signer;
    let sender: Keypair | Signer;
    let recipient: PublicKey;
    let mint: PublicKey;
    let mintAuthority: Keypair;

    let stateTreeInfo: TreeInfo;

    beforeAll(async () => {
        rpc = await getTestRpc(await WasmFactory.getInstance());
        payer = await newAccountWithLamports(rpc, 1e9);
        mintAuthority = Keypair.generate();
        const mintKeypair = Keypair.generate();

        stateTreeInfo = selectStateTreeInfo(await rpc.getStateTreeInfos());
        mint = (
            await createMint(
                rpc,
                payer,
                mintAuthority.publicKey,
                9,
                mintKeypair,
            )
        ).mint;
    });

    beforeEach(async () => {
        sender = await newAccountWithLamports(rpc, 1e9);
        recipient = (await newAccountWithLamports(rpc, 1e9)).publicKey;
    });

    it('should transfer using 4 accounts', async () => {
        // Mint specific amounts to create multiple token accounts
        await mintTo(
            rpc,
            payer,
            mint,
            sender.publicKey,
            mintAuthority,
            bn(25),
            stateTreeInfo,
        );
        await mintTo(
            rpc,
            payer,
            mint,
            sender.publicKey,
            mintAuthority,
            bn(25),
            stateTreeInfo,
        );
        await mintTo(
            rpc,
            payer,
            mint,
            sender.publicKey,
            mintAuthority,
            bn(25),
            stateTreeInfo,
        );
        await mintTo(
            rpc,
            payer,
            mint,
            sender.publicKey,
            mintAuthority,
            bn(25),
            stateTreeInfo,
        );

        const senderAccounts = await rpc.getCompressedTokenAccountsByOwner(
            sender.publicKey,
            { mint },
        );
        expect(senderAccounts.items.length).toBe(4);
        const totalAmount = senderAccounts.items.reduce(
            (sum, account) => sum.add(account.parsed.amount),
            bn(0),
        );
        expect(totalAmount.eq(bn(100))).toBe(true);

        const transferAmount = bn(100);

        await transferHelper(
            rpc,
            payer,
            mint,
            sender,
            transferAmount,
            recipient,
        );

        assertTransfer(
            rpc,
            senderAccounts.items,
            mint,
            transferAmount,
            sender.publicKey,
            recipient,
        );
    });
});

async function transferHelper(
    rpc: Rpc,
    payer: Signer,
    mint: PublicKey,
    owner: Signer,
    amount: BN,
    toAddress: PublicKey,
) {
    const compressedTokenAccounts = await rpc.getCompressedTokenAccountsByOwner(
        owner.publicKey,
        { mint },
    );

    const [inputAccounts] = selectMinCompressedTokenAccountsForTransfer(
        compressedTokenAccounts.items,
        amount,
    );

    const proof = await rpc.getValidityProof(
        inputAccounts.map(account => bn(account.compressedAccount.hash)),
    );

    const ix = await CompressedTokenProgram.transfer({
        payer: payer.publicKey,
        inputCompressedTokenAccounts: inputAccounts,
        toAddress,
        amount,
        recentInputStateRootIndices: proof.rootIndices,
        recentValidityProof: proof.compressedProof,
    });

    const { blockhash } = await rpc.getLatestBlockhash();
    const additionalSigners = dedupeSigner(payer, [owner]);
    const signedTx = buildAndSignTx(
        [ComputeBudgetProgram.setComputeUnitLimit({ units: 1_000_000 }), ix],
        payer,
        blockhash,
        additionalSigners,
    );

    const serializedTx = signedTx.serialize().length;

    expect(serializedTx).toBeLessThan(1000);

    const txId = await sendAndConfirmTx(rpc, signedTx);

    return txId;
}
