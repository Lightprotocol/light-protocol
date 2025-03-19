import { describe, it, expect, beforeAll } from 'vitest';
import { PublicKey, Keypair, Signer } from '@solana/web3.js';
import BN from 'bn.js';
import {
    ParsedTokenAccount,
    Rpc,
    bn,
    newAccountWithLamports,
    getTestRpc,
    StateTreeInfo,
    TreeType,
} from '@lightprotocol/stateless.js';
import { WasmFactory } from '@lightprotocol/hasher.rs';
import { createMint, decompress, mintTo } from '../../src/actions';
import { createAssociatedTokenAccount, getMint } from '@solana/spl-token';
import { getStateTreeInfoByTypeForTest } from '../../../stateless.js/tests/e2e/shared';

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

describe.each([TreeType.StateV1, TreeType.StateV2])(
    'decompress (treeType: %s)',
    treeType => {
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
            rpc = await getTestRpc(lightWasm);

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

            bob = await newAccountWithLamports(rpc, 1e9);
            charlie = await newAccountWithLamports(rpc, 1e9);

            charlieAta = await createAssociatedTokenAccount(
                rpc,
                payer,
                mint,
                charlie.publicKey,
            );

            await getMint(rpc, mint);

            await mintTo(
                rpc,
                payer,
                mint,
                bob.publicKey,
                mintAuthority,
                bn(1000),
                outputStateTreeInfo,
            );
        });

        const LOOP = 5;
        it(`should decompress from bob -> charlieAta ${LOOP} times`, async () => {
            const lightWasm = await WasmFactory.getInstance();
            rpc = await getTestRpc(lightWasm);

            for (let i = 0; i < LOOP; i++) {
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
                    bn(50),
                    bob,
                    charlieAta,
                    outputStateTreeInfo,
                );
                await assertDecompress(
                    rpc,
                    bn(recipientAtaBalanceBefore.value.amount),
                    charlieAta,
                    mint,
                    bn(50),
                    bob.publicKey,
                    senderCompressedTokenBalanceBefore.items,
                );
            }
        });
    },
);
