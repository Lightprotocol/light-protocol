import { describe, it, expect, beforeAll } from 'vitest';
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
    dedupeSigner,
    buildAndSignTx,
    sendAndConfirmTx,
    TreeInfo,
    selectStateTreeInfo,
} from '@lightprotocol/stateless.js';
import {
    createLiteSVMRpc,
    newAccountWithLamports,
    splCreateAssociatedTokenAccount,
} from '@lightprotocol/program-test';
import {
    compress,
    createMint,
    createTokenProgramLookupTable,
    decompress,
    mintTo,
} from '../../src/actions';
import { TOKEN_2022_PROGRAM_ID } from '@solana/spl-token';
import { CompressedTokenProgram } from '../../src/program';
import { NobleHasherFactory } from '@lightprotocol/program-test';
import {
    getTokenPoolInfos,
    selectTokenPoolInfo,
    TokenPoolInfo,
} from '../../src/utils/get-token-pool-infos';

/**
 * Assert that we created recipient and change ctokens for the sender, with all
 * amounts correctly accounted for
 */
async function assertCompress(
    rpc: Rpc,
    refSenderAtaBalanceBefore: BN,
    refSenderAta: PublicKey,
    refMint: PublicKey,
    refAmounts: BN[],
    refRecipients: PublicKey[],
    refRecipientCompressedTokenBalancesBefore: ParsedTokenAccount[][],
) {
    if (refAmounts.length !== refRecipients.length) {
        throw new Error('Mismatch in length of amounts and recipients arrays');
    }

    const refSenderAtaBalanceAfter =
        await rpc.getTokenAccountBalance(refSenderAta);

    const totalAmount = refAmounts.reduce((acc, curr) => acc.add(curr), bn(0));

    // Defensive type conversion: ensure amount is always a string before passing to bn()
    const afterAmountStr = String(refSenderAtaBalanceAfter.value.amount);
    console.log(
        '[TEST] assertCompress - refSenderAtaBalanceAfter.value.amount:',
        typeof refSenderAtaBalanceAfter.value.amount,
        refSenderAtaBalanceAfter.value.amount,
    );
    console.log(
        '[TEST] assertCompress - afterAmountStr:',
        typeof afterAmountStr,
        afterAmountStr,
    );
    console.log(
        '[TEST] assertCompress - refSenderAtaBalanceBefore:',
        refSenderAtaBalanceBefore.toString(),
    );
    console.log('[TEST] assertCompress - totalAmount:', totalAmount.toString());
    console.log(
        '[TEST] assertCompress - expected:',
        refSenderAtaBalanceBefore.sub(totalAmount).toString(),
    );

    expect(
        refSenderAtaBalanceBefore.sub(totalAmount).eq(bn(afterAmountStr)),
    ).toBe(true);

    for (let i = 0; i < refRecipients.length; i++) {
        const recipientCompressedTokenBalanceAfter =
            await rpc.getCompressedTokenAccountsByOwner(refRecipients[i], {
                mint: refMint,
            });

        const recipientSumPost =
            recipientCompressedTokenBalanceAfter.items.reduce(
                (acc, curr) => bn(acc).add(curr.parsed.amount),
                bn(0),
            );
        const recipientSumPre = refRecipientCompressedTokenBalancesBefore[
            i
        ].reduce((acc, curr) => bn(acc).add(curr.parsed.amount), bn(0));

        /// recipient should have received the amount
        expect(recipientSumPost.eq(refAmounts[i].add(recipientSumPre))).toBe(
            true,
        );
    }
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
    let lut: PublicKey;
    let stateTreeInfo: TreeInfo;
    let tokenPoolInfo: TokenPoolInfo;

    const maxBatchSize = 15;
    const recipients = Array.from(
        { length: maxBatchSize },
        () => Keypair.generate().publicKey,
    );

    beforeAll(async () => {
        const lightWasm = await NobleHasherFactory.getInstance();
        rpc = await createLiteSVMRpc(lightWasm);
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
        console.log('post mint');
        stateTreeInfo = selectStateTreeInfo(await rpc.getStateTreeInfos());
        console.log('post stateTreeInfo');
        tokenPoolInfo = selectTokenPoolInfo(await getTokenPoolInfos(rpc, mint));

        console.log('post tokenPoolInfo');
        bob = await newAccountWithLamports(rpc, 1e9);
        console.log('post bob');
        charlie = await newAccountWithLamports(rpc, 1e9);
        console.log('post charlie');

        bobAta = await splCreateAssociatedTokenAccount(
            rpc,
            payer,
            mint,
            bob.publicKey,
        );

        console.log('post bobAta');
        await mintTo(
            rpc,
            payer,
            mint,
            bob.publicKey,
            mintAuthority,
            bn(10000),
            stateTreeInfo,
            tokenPoolInfo,
        );

        console.log('post mintTo');
        await decompress(rpc, payer, mint, bn(900), bob, bobAta);

        console.log('post decompress');
        /// Setup LUT.
        const { address } = await createTokenProgramLookupTable(
            rpc,
            payer,
            payer,
            [mint],
            [
                payer.publicKey,
                bob.publicKey,
                bobAta,
                stateTreeInfo.tree,
                stateTreeInfo.queue,
            ],
        );
        lut = address;
        console.log('post lut');
    }, 80_000);

    it('should compress from bobAta -> charlie', async () => {
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
            stateTreeInfo,
            tokenPoolInfo,
        );
        // Defensive type conversion: ensure amount is always a string before passing to bn()
        await assertCompress(
            rpc,
            bn(String(senderAtaBalanceBefore.value.amount)),
            bobAta,
            mint,
            [bn(700)],
            [charlie.publicKey],
            [recipientCompressedTokenBalanceBefore.items],
        );
    });

    const amounts = Array.from({ length: maxBatchSize }, (_, i) => bn(i + 1));

    it('should compress to multiple (11 max without LUT) recipients with array of amounts and addresses', async () => {
        const senderAtaBalanceBefore = await rpc.getTokenAccountBalance(bobAta);

        const recipientCompressedTokenBalancesBefore = await Promise.all(
            recipients.map(recipient =>
                rpc.getCompressedTokenAccountsByOwner(recipient, { mint }),
            ),
        );

        // compress to 11 recipients
        await compress(
            rpc,
            payer,
            mint,
            amounts.slice(0, 11),
            bob,
            bobAta,
            recipients.slice(0, 11),
            stateTreeInfo,
            tokenPoolInfo,
        );

        // Defensive type conversion: ensure amount is always a string before passing to bn()
        for (let i = 0; i < recipients.length; i++) {
            await assertCompress(
                rpc,
                bn(String(senderAtaBalanceBefore.value.amount)),
                bobAta,
                mint,
                amounts.slice(0, 11),
                recipients.slice(0, 11),
                recipientCompressedTokenBalancesBefore.map(x => x.items),
            );
        }

        const senderAtaBalanceAfter = await rpc.getTokenAccountBalance(bobAta);
        const totalCompressed = amounts
            .slice(0, 11)
            .reduce((sum, amount) => sum.add(amount), bn(0));

        // Defensive type conversion: ensure amount is always a string before passing to bn()
        const beforeAmount = String(senderAtaBalanceBefore.value.amount);
        const afterAmount = String(senderAtaBalanceAfter.value.amount);
        console.log(
            '[TEST] compress.test - beforeAmount:',
            typeof beforeAmount,
            beforeAmount,
        );
        console.log(
            '[TEST] compress.test - afterAmount:',
            typeof afterAmount,
            afterAmount,
        );
        console.log(
            '[TEST] compress.test - totalCompressed:',
            totalCompressed.toString(),
        );

        expect(afterAmount).toEqual(
            bn(beforeAmount).sub(totalCompressed).toString(),
        );
    });

    it('should fail when passing unequal array lengths for amounts and toAddress', async () => {
        await expect(
            compress(
                rpc,
                payer,
                mint,
                amounts.slice(0, 10),
                bob,
                bobAta,
                recipients.slice(0, 11),
                stateTreeInfo,
                tokenPoolInfo,
            ),
        ).rejects.toThrow(
            'Amount and toAddress arrays must have the same length',
        );

        await expect(
            compress(
                rpc,
                payer,
                mint,
                amounts[0],
                bob,
                bobAta,
                recipients,
                stateTreeInfo,
                tokenPoolInfo,
            ),
        ).rejects.toThrow(
            'Amount and toAddress arrays must have the same length',
        );
    });

    // Doesnt work in litesvm
    it.skip(`should compress-batch to max ${maxBatchSize} recipients optimized with LUT`, async () => {
        /// Fetch state of LUT
        const lookupTableAccount = (await rpc.getAddressLookupTable(lut))
            .value!;

        /// Compress to max recipients with LUT
        const ix = await CompressedTokenProgram.compress({
            payer: bob.publicKey,
            owner: bob.publicKey,
            source: bobAta,
            toAddress: recipients,
            amount: recipients.map(() => bn(2)),
            mint,
            outputStateTreeInfo: stateTreeInfo,
            tokenPoolInfo,
        });

        const { blockhash } = await rpc.getLatestBlockhash();
        const additionalSigners = dedupeSigner(payer, [bob]);

        const tx = buildAndSignTx(
            [ComputeBudgetProgram.setComputeUnitLimit({ units: 500_000 }), ix],
            payer,
            blockhash,
            additionalSigners,
            [lookupTableAccount],
        );
        await sendAndConfirmTx(rpc, tx);
    });

    it('should compress from bob Token 2022 Ata -> charlie', async () => {
        const mintKeypair = Keypair.generate();

        const token22Mint = (
            await createMint(
                rpc,
                payer,
                mintAuthority.publicKey,
                TEST_TOKEN_DECIMALS, // decimals
                mintKeypair, // keypair
                undefined, // confirmOptions
                TOKEN_2022_PROGRAM_ID, // tokenProgramId
                undefined, // freezeAuthority
            )
        ).mint;
        const mintAccountInfo = await rpc.getAccountInfo(token22Mint);
        expect(mintAccountInfo!.owner.equals(TOKEN_2022_PROGRAM_ID)).toBe(true);

        bob = await newAccountWithLamports(rpc, 1e9);
        charlie = await newAccountWithLamports(rpc, 1e9);

        const bobToken2022Ata = await splCreateAssociatedTokenAccount(
            rpc,
            payer,
            token22Mint,
            bob.publicKey,
            TOKEN_2022_PROGRAM_ID,
        );

        const tokenPoolInfoT22 = selectTokenPoolInfo(
            await getTokenPoolInfos(rpc, token22Mint),
        );

        await expect(
            mintTo(
                rpc,
                payer,
                token22Mint,
                bob.publicKey,
                mintAuthority,
                bn(10000),
                stateTreeInfo,
                tokenPoolInfo,
            ),
        ).rejects.toThrow();

        await mintTo(
            rpc,
            payer,
            token22Mint,
            bob.publicKey,
            mintAuthority,
            bn(10000),
            stateTreeInfo,
            tokenPoolInfoT22,
        );
        await decompress(
            rpc,
            payer,
            token22Mint,
            bn(9000),
            bob,
            bobToken2022Ata,
        );
        const senderAtaBalanceBefore =
            await rpc.getTokenAccountBalance(bobToken2022Ata);
        const recipientCompressedTokenBalanceBefore =
            await rpc.getCompressedTokenAccountsByOwner(charlie.publicKey, {
                mint: token22Mint,
            });

        await compress(
            rpc,
            payer,
            token22Mint,
            bn(701),
            bob,
            bobToken2022Ata,
            charlie.publicKey,
            stateTreeInfo,
            tokenPoolInfoT22,
        );
        // Defensive type conversion: ensure amount is always a string before passing to bn()
        await assertCompress(
            rpc,
            bn(String(senderAtaBalanceBefore.value.amount)),
            bobToken2022Ata,
            token22Mint,
            [bn(701)],
            [charlie.publicKey],
            [recipientCompressedTokenBalanceBefore.items],
        );
    });
});
