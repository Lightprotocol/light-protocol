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
} from '@lightprotocol/stateless.js';
import { WasmFactory } from '@lightprotocol/hasher.rs';
import { createMint, freeze, mintTo, thaw } from '../../src/actions';
import { createAssociatedTokenAccount } from '@solana/spl-token';

const TEST_TOKEN_DECIMALS = 2;

describe('freeze and thaw', () => {
    let rpc: Rpc;
    let payer: Signer;
    let bob: Signer;

    let charlie: Signer;
    let charlieAta: PublicKey;
    let mint: PublicKey;
    let mintAuthority: Keypair;
    let freezeAuthority: Keypair;
    const { merkleTree } = defaultTestStateTreeAccounts();

    beforeAll(async () => {
        const lightWasm = await WasmFactory.getInstance();
        rpc = await getTestRpc(lightWasm);
        payer = await newAccountWithLamports(rpc, 1e9);
        mintAuthority = Keypair.generate();
        const mintKeypair = Keypair.generate();
        freezeAuthority = Keypair.generate();

        mint = (
            await createMint(
                rpc,
                payer,
                mintAuthority.publicKey,
                TEST_TOKEN_DECIMALS,
                mintKeypair,
                undefined,
                undefined,
                freezeAuthority.publicKey,
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

    const LOOP = 1;
    it(`should freeze and thaw token account ${LOOP} times`, async () => {
        const lightWasm = await WasmFactory.getInstance();
        rpc = await getTestRpc(lightWasm);
        for (let i = 0; i < LOOP; i++) {
            // const recipientAtaBalanceBefore =
            //     await rpc.getTokenAccountBalance(charlieAta);
            const senderCompressedTokenBalanceBefore =
                await rpc.getCompressedTokenAccountsByOwner(bob.publicKey, {
                    mint,
                });

            const txId = await freeze(
                rpc,
                payer,
                senderCompressedTokenBalanceBefore.items,
                mint,
                freezeAuthority,
                merkleTree,
            );
            console.log('txId', txId);

            // const thawTxId = await thaw(
            //     rpc,
            //     payer,
            //     senderCompressedTokenBalanceBefore.items,
            //     mint,
            //     freezeAuthority,
            //     merkleTree,
            // );
            // console.log('thawTxId', thawTxId);
        }
    });
});
