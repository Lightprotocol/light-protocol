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
import { NobleHasherFactory } from '@lightprotocol/program-test';
import {
    getTokenPoolInfos,
    selectTokenPoolInfo,
    TokenPoolInfo,
} from '../../src/utils/get-token-pool-infos';
import { getTestRpc } from '@lightprotocol/program-test';

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
        const lightWasm = await NobleHasherFactory.getInstance();
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
});
