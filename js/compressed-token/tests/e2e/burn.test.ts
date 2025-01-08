import { describe, it, expect, beforeAll } from 'vitest';
import { PublicKey, Keypair, Signer } from '@solana/web3.js';
import BN from 'bn.js';
import {
    ParsedTokenAccount,
    Rpc,
    bn,
    newAccountWithLamports,
    getTestRpc,
} from '@lightprotocol/stateless.js';
import { WasmFactory } from '@lightprotocol/hasher.rs';
import { burn, createMint, mintTo } from '../../src/actions';
import { createAssociatedTokenAccount } from '@solana/spl-token';

// /**
//  * Assert that we created recipient and change ctokens for the sender, with all
//  * amounts correctly accounted for
//  */
async function assertBurn(
    rpc: Rpc,
    refMint: PublicKey,
    refAmount: BN,
    refSender: PublicKey,
    refSenderCompressedTokenBalanceBefore: ParsedTokenAccount[],
) {
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

    /// should have burned the amount
    expect(senderSumPost.eq(senderSumPre.sub(refAmount))).toBe(true);
}

const TEST_TOKEN_DECIMALS = 2;

describe('burn', () => {
    let rpc: Rpc;
    let payer: Signer;
    let bob: Signer;

    let charlie: Signer;
    let charlieAta: PublicKey;
    let mint: PublicKey;
    let mintAuthority: Keypair;

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

        charlieAta = await createAssociatedTokenAccount(
            rpc,
            payer,
            mint,
            charlie.publicKey,
        );

        await mintTo(rpc, payer, mint, bob.publicKey, mintAuthority, bn(1000));
    });

    it(`should burn all from bob`, async () => {
        const lightWasm = await WasmFactory.getInstance();
        rpc = await getTestRpc(lightWasm);

        const bobCompressedTokenBalanceBefore =
            await rpc.getCompressedTokenAccountsByOwner(bob.publicKey, {
                mint,
            });

        const txId = await burn(rpc, bob, mint, bn(500), bob);
        await assertBurn(
            rpc,
            mint,
            bn(500),
            bob.publicKey,
            bobCompressedTokenBalanceBefore.items,
        );

        const bobCompressedTokenBalanceBefore2 =
            await rpc.getCompressedTokenAccountsByOwner(bob.publicKey, {
                mint,
            });
        const txId2 = await burn(rpc, payer, mint, bn(500), bob);
        await assertBurn(
            rpc,
            mint,
            bn(500),
            bob.publicKey,
            bobCompressedTokenBalanceBefore2.items,
        );
    });
});
