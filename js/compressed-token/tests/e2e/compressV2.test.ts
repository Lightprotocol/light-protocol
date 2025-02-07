import { describe, it, expect, beforeAll } from 'vitest';
import {
    PublicKey,
    Keypair,
    Signer,
    ComputeBudgetProgram,
    SystemProgram,
} from '@solana/web3.js';
import BN from 'bn.js';
import {
    ParsedTokenAccount,
    Rpc,
    bn,
    defaultTestStateTreeAccounts,
    newAccountWithLamports,
    dedupeSigner,
    buildAndSignTx,
    sendAndConfirmTx,
    getTestRpc,
} from '@lightprotocol/stateless.js';
import {
    compressV2,
    createMint,
    createTokenProgramLookupTable,
    decompress,
    mintTo,
} from '../../src/actions';
import {
    createAssociatedTokenAccount,
    TOKEN_2022_PROGRAM_ID,
    TOKEN_PROGRAM_ID,
} from '@solana/spl-token';
import { CompressedTokenProgram } from '../../src/program';
import { WasmFactory } from '@lightprotocol/hasher.rs';

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
    let amount: BN;
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
            defaultTestStateTreeAccounts().merkleTree,
        );

        await decompress(rpc, payer, mint, bn(9000), bob, bobAta);

        /// Setup LUT.
        const { address } = await createTokenProgramLookupTable(
            rpc,
            payer,
            payer,
            [mint],
            [bob.publicKey, bobAta, payer.publicKey, SystemProgram.programId],
        );
        lut = address;
        console.log('lut', lut.toBase58());
    }, 80_000);

    const maxBatchSize = 33;
    const recipients = Array.from(
        { length: maxBatchSize },
        () => Keypair.generate().publicKey,
    );

    it('should compressv2 to many', async () => {
        const senderAtaBalanceBefore = await rpc.getTokenAccountBalance(bobAta);

        const recipientCompressedTokenBalancesBefore = await Promise.all(
            recipients.map(recipient =>
                rpc.getCompressedTokenAccountsByOwner(recipient, { mint }),
            ),
        );

        amount = bn(30);

        const txId = await compressV2(
            rpc,
            payer,
            mint,
            recipients.slice(0, 26),
            bob,
            bobAta,
            amount,
            defaultTestStateTreeAccounts().merkleTree,
            undefined,
            undefined,
            lut,
        );
        console.log('txId', txId);

        // for (let i = 0; i < recipients.length; i++) {
        //     await assertCompress(
        //         rpc,
        //         bn(senderAtaBalanceBefore.value.amount),
        //         bobAta,
        //         mint,
        //         [amount],
        //         recipients,
        //         recipientCompressedTokenBalancesBefore.map(x => x.items),
        //     );
        // }

        // const senderAtaBalanceAfter = await rpc.getTokenAccountBalance(bobAta);
        // const totalCompressed = amounts
        //     .slice(0, 11)
        //     .reduce((sum, amount) => sum.add(amount), bn(0));
        // expect(senderAtaBalanceAfter.value.amount).toEqual(
        //     bn(senderAtaBalanceBefore.value.amount)
        //         .sub(totalCompressed)
        //         .toString(),
        // );
    });
});
