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
    newAccountWithLamports,
    dedupeSigner,
    buildAndSignTx,
    sendAndConfirmTx,
    getTestRpc,
    TreeInfo,
    selectStateTreeInfo,
} from '@lightprotocol/stateless.js';
import {
    compress,
    createMint,
    createTokenProgramLookupTable,
    decompress,
    mintTo,
} from '../../src/actions';
import {
    createAssociatedTokenAccount,
    TOKEN_2022_PROGRAM_ID,
} from '@solana/spl-token';
import { CompressedTokenProgram } from '../../src/program';
import { WasmFactory } from '@lightprotocol/hasher.rs';
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

    expect(
        refSenderAtaBalanceBefore
            .sub(totalAmount)
            .eq(bn(refSenderAtaBalanceAfter.value.amount)),
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

        stateTreeInfo = selectStateTreeInfo(await rpc.getStateTreeInfos());
        tokenPoolInfo = selectTokenPoolInfo(await getTokenPoolInfos(rpc, mint));

        bob = await newAccountWithLamports(rpc, 1e9);
        charlie = await newAccountWithLamports(rpc, 1e9);

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
            bn(10000),
            stateTreeInfo,
            tokenPoolInfo,
        );

        await decompress(rpc, payer, mint, bn(900), bob, bobAta);

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
        await assertCompress(
            rpc,
            bn(senderAtaBalanceBefore.value.amount),
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

        for (let i = 0; i < recipients.length; i++) {
            await assertCompress(
                rpc,
                bn(senderAtaBalanceBefore.value.amount),
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
        expect(senderAtaBalanceAfter.value.amount).toEqual(
            bn(senderAtaBalanceBefore.value.amount)
                .sub(totalCompressed)
                .toString(),
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

    it(`should compress-batch to max ${maxBatchSize} recipients optimized with LUT`, async () => {
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
                TEST_TOKEN_DECIMALS,
                mintKeypair,
                undefined,
                true,
            )
        ).mint;
        const mintAccountInfo = await rpc.getAccountInfo(token22Mint);
        expect(
            mintAccountInfo!.owner.toBase58(),
            TOKEN_2022_PROGRAM_ID.toBase58(),
        );

        bob = await newAccountWithLamports(rpc, 1e9);
        charlie = await newAccountWithLamports(rpc, 1e9);

        const bobToken2022Ata = await createAssociatedTokenAccount(
            rpc,
            payer,
            token22Mint,
            bob.publicKey,
            undefined,
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
        await assertCompress(
            rpc,
            bn(senderAtaBalanceBefore.value.amount),
            bobToken2022Ata,
            token22Mint,
            [bn(701)],
            [charlie.publicKey],
            [recipientCompressedTokenBalanceBefore.items],
        );
    });
});
