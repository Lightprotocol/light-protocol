import { describe, it, expect, beforeAll } from 'vitest';
import { Connection, PublicKey, Signer, Keypair } from '@solana/web3.js';
import { unpackMint, unpackAccount } from '@solana/spl-token';
import { BN } from '@coral-xyz/anchor';
import { bn, defaultTestStateTreeAccounts } from '@lightprotocol/stateless.js';
import { CompressedTokenProgram } from '../../src/program';
import { createMint, mintTo } from '../../src/actions';
import { getCompressedTokenAccountsFromMockRpc } from '../../src/token-serde';
import { getConnection, newAccountWithLamports } from './common';

/**
 * Asserts that mintTo() creates a new compressed token account for the
 * recipient
 */
async function assertMintTo(
    connection: Connection,
    refMint: PublicKey,
    refAmount: BN,
    refTo: PublicKey,
) {
    const compressedTokenAccounts = await getCompressedTokenAccountsFromMockRpc(
        connection,
        refTo,
        refMint,
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
    let connection: Connection;
    let payer: Signer;
    let bob: Signer;
    let mint: PublicKey;
    let mintAuthority: Keypair;
    const { merkleTree } = defaultTestStateTreeAccounts();

    beforeAll(async () => {
        connection = getConnection();
        payer = await newAccountWithLamports(connection);
        mintAuthority = Keypair.generate();
        const mintKeypair = Keypair.generate();

        mint = (
            await createMint(
                connection,
                payer,
                mintAuthority.publicKey,
                TEST_TOKEN_DECIMALS,
                mintKeypair,
            )
        ).mint;
    });

    it('should mint to bob', async () => {
        const amount = bn(1000);
        await mintTo(
            connection,
            payer,
            mint,
            bob.publicKey,
            mintAuthority,
            amount,
            [],
        );

        await assertMintTo(connection, mint, amount, bob.publicKey);

        /// wrong authority
        await expect(
            mintTo(connection, payer, mint, bob.publicKey, payer, amount, []),
        ).rejects.toThrow();

        /// with output state merkle tree defined
        await mintTo(
            connection,
            payer,
            mint,
            bob.publicKey,
            payer,
            amount,
            [],
            merkleTree,
        );
    });
});
