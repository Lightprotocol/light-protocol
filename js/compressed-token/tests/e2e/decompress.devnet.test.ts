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
    createRpc,
} from '@lightprotocol/stateless.js';
import { WasmFactory } from '@lightprotocol/hasher.rs';
import { createMint, decompress, mintTo } from '../../src/actions';
import { createAssociatedTokenAccount } from '@solana/spl-token';
import * as dotenv from 'dotenv';
import bs58 from 'bs58';
dotenv.config();

const RPC_URL = process.env.RPC_URL;
const PAYER_KEYPAIR = Keypair.fromSecretKey(
    bs58.decode(process.env.PAYER_KEYPAIR!),
);
if (!RPC_URL) {
    throw new Error('RPC_URL is not defined in the .env file');
}

/**
 * Assert that we created recipient and change ctokens for the sender, with all
 * amounts correctly accounted for
 */
async function assertDecompress(
    rpc: Rpc,
    refRecipientAtaBalanceBefore: BN,
    refRecipientAta: PublicKey, // all
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

    /// recipient ata should have received the amount
    expect(
        bn(refRecipientAtaBalanceAfter.value.amount)
            .sub(refAmount)
            .eq(refRecipientAtaBalanceBefore),
    ).toBe(true);

    /// should have sent the amount
    expect(senderSumPost.eq(senderSumPre.sub(refAmount))).toBe(true);
}

const TEST_TOKEN_DECIMALS = 2;

describe('decompress', () => {
    let rpc: Rpc;
    let payer: Signer;
    let bob: Signer;

    let charlie: Signer;
    let charlieAta: PublicKey;
    let mint: PublicKey;
    let mintAuthority: Keypair;
    const { merkleTree } = defaultTestStateTreeAccounts();

    // Devnet debug.
    mint = new PublicKey('GUzLf9hdWfjnTzzuLQCEmkj9GBvyBEdskMHWxQ2F7EWs');
    charlieAta = new PublicKey('GGnXCQggBESkHCoHEPVbuch5JGSDn36h9W73RTaQDG7T');
    rpc = createRpc(RPC_URL);
    payer = PAYER_KEYPAIR;
    bob = Keypair.fromSecretKey(
        new Uint8Array(JSON.parse(process.env.BOB_KEYPAIR!)),
    );
    charlie = Keypair.fromSecretKey(
        new Uint8Array(JSON.parse(process.env.CHARLIE_KEYPAIR!)),
    );
    console.log('PAYER:', payer.publicKey.toBase58());
    console.log('BOB:', bob.publicKey.toBase58());
    console.log('CHARLIE:', charlie.publicKey.toBase58());

    // beforeAll(async () => {
    //     const lightWasm = await WasmFactory.getInstance();

    //     mintAuthority = Keypair.generate();
    //     const mintKeypair = Keypair.generate();

    //     mint = (
    //         await createMint(
    //             rpc,
    //             payer,
    //             mintAuthority.publicKey,
    //             TEST_TOKEN_DECIMALS,
    //             mintKeypair,
    //         )
    //     ).mint;
    //     console.log('MINT:', mint.toBase58());

    //     charlieAta = await createAssociatedTokenAccount(
    //         rpc,
    //         payer,
    //         mint,
    //         charlie.publicKey,
    //     );
    //     console.log('CHARLIE ATA:', charlieAta.toBase58());

    //     const res = await mintTo(
    //         rpc,
    //         payer,
    //         mint,
    //         bob.publicKey,
    //         mintAuthority,
    //         bn(1000),
    //         defaultTestStateTreeAccounts().merkleTree,
    //     );
    //     console.log('MINTED TO BOB', res);
    //     throw new Error('Not implemented');
    // });

    const LOOP = 3;
    it(`should decompress from bob -> charlieAta ${LOOP} times`, async () => {
        for (let i = 0; i < LOOP; i++) {
            const recipientAtaBalanceBefore =
                await rpc.getTokenAccountBalance(charlieAta);
            const senderCompressedTokenBalanceBefore =
                await rpc.getCompressedTokenAccountsByOwner(bob.publicKey, {
                    mint,
                });

            const merkleTree2Pubkey =
                'smt2rJAFdyJJupwMKAqTNAJwvjhmiZ4JYGZmbVRw1Ho';
            const nullifierQueue2Pubkey =
                'nfq2hgS7NYemXsFaFUCe3EMXSDSfnZnAe27jC6aPP1X';

            const res = await decompress(
                rpc,
                payer,
                mint,
                bn(15),
                bob,
                charlieAta,
                new PublicKey(merkleTree2Pubkey),
            );
            console.log('res:', res);

            await assertDecompress(
                rpc,
                bn(recipientAtaBalanceBefore.value.amount),
                charlieAta,
                mint,
                bn(15),
                bob.publicKey,
                senderCompressedTokenBalanceBefore.items,
            );
        }
    });
});
