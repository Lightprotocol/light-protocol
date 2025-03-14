import { describe, it, expect, beforeAll } from 'vitest';
import { PublicKey, Keypair, Signer } from '@solana/web3.js';
import BN from 'bn.js';
import {
    ParsedTokenAccount,
    Rpc,
    bn,
    defaultTestStateTreeAccounts,
    newAccountWithLamports,
    getTestRpc,
    StateTreeInfo,
    TreeType,
    createRpc,
} from '@lightprotocol/stateless.js';
import { WasmFactory } from '@lightprotocol/hasher.rs';
import { createMint, decompress, mintTo } from '../../src/actions';
import { createAssociatedTokenAccount, getMint } from '@solana/spl-token';
import { getStateTreeInfoByTypeForTest } from '../../../stateless.js/tests/e2e/shared';
import { CompressedTokenProgram } from '../../src/program';

/**
 * Assert that we created recipient and change ctokens for the sender, with all
 * amounts correctly accounted for
 */
async function assertDecompress(
    rpc: Rpc,
    refRecipientAtaBalanceBefore: BN,
    refRecipientAta: PublicKey,
    refMint: PublicKey,
    refAmount: BN,
    refSender: PublicKey,
    refSenderCompressedTokenBalanceBefore: ParsedTokenAccount[],
) {
    const refRecipientAtaBalanceAfter =
        await rpc.getTokenAccountBalance(refRecipientAta);

    const senderCompressedTokenBalanceAfter = (
        await rpc.getCompressedTokenAccountsByOwner(refSender, {
            mint: refMint,
        })
    ).items;

    const senderSumPost = senderCompressedTokenBalanceAfter.reduce(
        (acc, curr) => bn(acc).add(curr.parsed.amount),
        bn(0),
    );
    const senderSumPre = refSenderCompressedTokenBalanceBefore.reduce(
        (acc, curr) => bn(acc).add(curr.parsed.amount),
        bn(0),
    );

    expect(
        bn(refRecipientAtaBalanceAfter.value.amount)
            .sub(refAmount)
            .eq(refRecipientAtaBalanceBefore),
    ).toBe(true);

    expect(senderSumPost.eq(senderSumPre.sub(refAmount))).toBe(true);
}

const TEST_TOKEN_DECIMALS = 2;

describe.each([TreeType.StateV1])('decompress with state tree %s', treeType => {
    let rpc: Rpc;
    let payer: Signer;
    let bob: Signer;
    let charlie: Signer;
    let charlieAta: PublicKey;
    let mint: PublicKey;
    let mintAuthority: Keypair;
    let outputStateTreeInfo: StateTreeInfo;

    beforeAll(async () => {
        const lightWasm = await WasmFactory.getInstance();
        // rpc = await getTestRpc(lightWasm);
        rpc = createRpc();
        outputStateTreeInfo = await getStateTreeInfoByTypeForTest(
            rpc,
            treeType,
        );

        payer = await newAccountWithLamports(rpc, 1e9, 256);
        mintAuthority = Keypair.generate();
        const mintKeypair = Keypair.generate();

        const mintTx = await createMint(
            rpc,
            payer,
            mintAuthority.publicKey,
            TEST_TOKEN_DECIMALS,
            mintKeypair,
        );
        mint = mintTx.mint;
        console.log('mint txId', mintTx.transactionSignature);

        bob = await newAccountWithLamports(rpc, 1e9);
        charlie = await newAccountWithLamports(rpc, 1e9);

        charlieAta = await createAssociatedTokenAccount(
            rpc,
            payer,
            mint,
            charlie.publicKey,
        );
        console.log('charlieAta', charlieAta.toBase58());

        const tokenPoolPda = CompressedTokenProgram.deriveTokenPoolPda(mint);
        const tokenPoolPdaTokenBalanceBefore =
            await rpc.getTokenAccountBalance(tokenPoolPda);
        console.log(
            'tokenPoolPdaTokenBalanceBefore',
            tokenPoolPdaTokenBalanceBefore,
        );

        const mintInfo = await getMint(rpc, mint);
        console.log(
            'Total supply of tokens onchain BEFORE:',
            mintInfo.supply.toString(),
        );

        const txId2 = await mintTo(
            rpc,
            payer,
            mint,
            bob.publicKey,
            mintAuthority,
            bn(1000),
            outputStateTreeInfo,
        );
        console.log('mintTo txId', txId2);

        const tokenPoolPdaTokenBalanceAfter =
            await rpc.getTokenAccountBalance(tokenPoolPda);
        console.log(
            'tokenPoolPdaTokenBalanceAfter',
            tokenPoolPdaTokenBalanceAfter,
        );

        const mintInfo2 = await getMint(rpc, mint);
        console.log(
            'Total supply of tokens onchain AFTER:',
            mintInfo2.supply.toString(),
        );
    });

    const LOOP = 1;
    it(`should decompress from bob -> charlieAta ${LOOP} times`, async () => {
        const lightWasm = await WasmFactory.getInstance();
        rpc = await getTestRpc(lightWasm);
        rpc = createRpc();
        for (let i = 0; i < LOOP; i++) {
            const recipientAtaBalanceBefore =
                await rpc.getTokenAccountBalance(charlieAta);
            const senderCompressedTokenBalanceBefore =
                await rpc.getCompressedTokenAccountsByOwner(bob.publicKey, {
                    mint,
                });

            console.log(
                'senderCompressedTokenBalance BEFORE DECOMPRESS',
                senderCompressedTokenBalanceBefore.items
                    .reduce((acc, curr) => acc.add(curr.parsed.amount), bn(0))
                    .toString(),
            );
            console.log('recipientAtaBalanceBefore', recipientAtaBalanceBefore);
            console.log(
                'bob lamport balance',
                await rpc.getBalance(bob.publicKey),
            );
            console.log(
                'charlieAta lamport balance',
                await rpc.getBalance(charlieAta),
            );
            console.log(
                'payer lamport balance',
                await rpc.getBalance(payer.publicKey),
            );

            const txId = await decompress(
                rpc,
                payer,
                mint,
                bn(1000),
                bob,
                charlieAta,
                outputStateTreeInfo,
            );
            console.log('txId', txId);
            await assertDecompress(
                rpc,
                bn(recipientAtaBalanceBefore.value.amount),
                charlieAta,
                mint,
                bn(1000),
                bob.publicKey,
                senderCompressedTokenBalanceBefore.items,
            );
        }
    });
});
