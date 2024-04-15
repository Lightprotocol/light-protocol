import { describe, it, expect, beforeAll } from 'vitest';
import { PublicKey, Signer, Keypair } from '@solana/web3.js';
import { BN } from '@coral-xyz/anchor';
import { createMint, mintTo } from '../../src/actions';
import {
    getTestKeypair,
    newAccountWithLamports,
    bn,
    defaultTestStateTreeAccounts,
    createRpc,
    Rpc,
} from '@lightprotocol/stateless.js';

/**
 * Asserts that mintTo() creates a new compressed token account for the
 * recipient
 */
async function assertMintTo(
    rpc: Rpc,
    refMint: PublicKey,
    refAmount: BN,
    refTo: PublicKey,
) {
    const compressedTokenAccounts = await rpc.getCompressedTokenAccountsByOwner(
        refTo,
        {
            mint: refMint,
        },
    );

    const compressedTokenAccount = compressedTokenAccounts[0];
    expect(compressedTokenAccount.parsed.mint.toBase58()).toBe(
        refMint.toBase58(),
    );
    expect(compressedTokenAccount.parsed.amount.eq(refAmount)).toBe(true);
    expect(compressedTokenAccount.parsed.owner.equals(refTo)).toBe(true);
    expect(compressedTokenAccount.parsed.delegate).toBe(null);
}

const TEST_TOKEN_DECIMALS = 2;

describe('mintTo', () => {
    let rpc: Rpc;
    let payer: Signer;
    let bob: Signer;
    let mint: PublicKey;
    let mintAuthority: Keypair;
    const { merkleTree } = defaultTestStateTreeAccounts();

    beforeAll(async () => {
        rpc = createRpc();
        payer = await newAccountWithLamports(rpc);
        bob = getTestKeypair();
        mintAuthority = Keypair.generate();
        const mintKeypair = Keypair.generate();

        mint = (
            await createMint(
                rpc,
                payer,
                mintAuthority,
                TEST_TOKEN_DECIMALS,
                mintKeypair,
            )
        ).mint;
    });

    it('should mint to bob', async () => {
        const amount = bn(1000);
        await mintTo(rpc, payer, mint, bob.publicKey, mintAuthority, amount);

        await assertMintTo(rpc, mint, amount, bob.publicKey);

        /// wrong authority
        await expect(
            mintTo(rpc, payer, mint, bob.publicKey, payer, amount),
        ).rejects.toThrowError(/custom program error: 0x7d3/);

        /// with output state merkle tree defined
        await mintTo(
            rpc,
            payer,
            mint,
            bob.publicKey,
            mintAuthority,
            amount,
            merkleTree,
        );
    });
});
