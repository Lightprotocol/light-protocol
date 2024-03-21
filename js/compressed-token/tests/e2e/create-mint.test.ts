import { describe, it, expect, beforeAll, assert } from 'vitest';
import { CompressedTokenProgram } from '../../src/program';
import { Connection, PublicKey, Signer, Keypair } from '@solana/web3.js';
import { unpackMint, unpackAccount } from '@solana/spl-token';
import { createMint } from '../../src/actions';
import { getConnection, newAccountWithLamports } from './common';

/**
 * Asserts that createMint() creates a new spl mint account + the respective
 * system pool account
 */
async function assertCreateMint(
    mint: PublicKey,
    authority: PublicKey,
    connection: Connection,
    decimals: number,
    poolAccount: PublicKey,
) {
    const mintAcc = await connection.getAccountInfo(mint);
    const unpackedMint = unpackMint(mint, mintAcc);

    const mintAuthority = CompressedTokenProgram.deriveMintAuthorityPda(
        authority,
        mint,
    );

    expect(unpackedMint.mintAuthority?.toString()).toBe(
        mintAuthority.toString(),
    );
    expect(unpackedMint.supply).toBe(0n);
    expect(unpackedMint.decimals).toBe(decimals);
    expect(unpackedMint.isInitialized).toBe(true);
    expect(unpackedMint.freezeAuthority).toBe(null);
    expect(unpackedMint.tlvData.length).toBe(0);

    /// Pool (omnibus) account is a regular SPL Token account
    const poolAccountInfo = await connection.getAccountInfo(poolAccount);
    const unpackedPoolAccount = unpackAccount(poolAccount, poolAccountInfo);
    expect(unpackedPoolAccount.mint.equals(mint)).toBe(true);
    expect(unpackedPoolAccount.amount).toBe(0n);
    expect(unpackedPoolAccount.owner.equals(mintAuthority)).toBe(true);
    expect(unpackedPoolAccount.delegate).toBe(null);
}

const TEST_TOKEN_DECIMALS = 2;

describe('createMint', () => {
    let connection: Connection;
    let payer: Signer;
    let mint: PublicKey;
    let mintAuthority: Keypair;

    beforeAll(async () => {
        connection = getConnection();
        payer = await newAccountWithLamports(connection);
    });

    it('should create mint', async () => {
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
        const poolAccount = CompressedTokenProgram.deriveTokenPoolPda(mint);

        assert(mint.equals(mintKeypair.publicKey));

        await assertCreateMint(
            mint,
            mintAuthority.publicKey,
            connection,
            TEST_TOKEN_DECIMALS,
            poolAccount,
        );

        /// Mint already exists
        await expect(
            await createMint(
                connection,
                payer,
                mintAuthority.publicKey,
                TEST_TOKEN_DECIMALS,
                mintKeypair,
            ),
        ).rejects.toThrow();
    });

    it('should create mint with payer as authority', async () => {
        mint = (
            await createMint(
                connection,
                payer,
                payer.publicKey,
                TEST_TOKEN_DECIMALS,
                // random mint
            )
        ).mint;

        const poolAccount = CompressedTokenProgram.deriveTokenPoolPda(mint);

        await assertCreateMint(
            mint,
            payer.publicKey,
            connection,
            TEST_TOKEN_DECIMALS,
            poolAccount,
        );
    });
});
