import { describe, it, expect, beforeAll, assert } from 'vitest';
import { CompressedTokenProgram } from '../../src/program';
import { PublicKey, Signer, Keypair } from '@solana/web3.js';
import { unpackMint, unpackAccount } from '@solana/spl-token';
import { createMint } from '../../src/actions';
import {
    Rpc,
    newAccountWithLamports,
    getTestRpc,
} from '@lightprotocol/stateless.js';
import { WasmFactory } from '@lightprotocol/hasher.rs';

/**
 * Asserts that createMint() creates a new spl mint account + the respective
 * system pool account
 */
async function assertCreateMint(
    mint: PublicKey,
    authority: PublicKey,
    rpc: Rpc,
    decimals: number,
    poolAccount: PublicKey,
) {
    const mintAcc = await rpc.getAccountInfo(mint);
    const unpackedMint = unpackMint(mint, mintAcc);

    expect(unpackedMint.mintAuthority?.toString()).toBe(authority.toString());
    expect(unpackedMint.supply).toBe(0n);
    expect(unpackedMint.decimals).toBe(decimals);
    expect(unpackedMint.isInitialized).toBe(true);
    expect(unpackedMint.freezeAuthority).toBe(null);
    expect(unpackedMint.tlvData.length).toBe(0);

    /// Pool (omnibus) account is a regular SPL Token account
    const poolAccountInfo = await rpc.getAccountInfo(poolAccount);
    const unpackedPoolAccount = unpackAccount(poolAccount, poolAccountInfo);
    expect(unpackedPoolAccount.mint.equals(mint)).toBe(true);
    expect(unpackedPoolAccount.amount).toBe(0n);
    expect(
        unpackedPoolAccount.owner.equals(
            CompressedTokenProgram.deriveCpiAuthorityPda,
        ),
    ).toBe(true);
    expect(unpackedPoolAccount.delegate).toBe(null);
}

const TEST_TOKEN_DECIMALS = 2;
describe('createMint', () => {
    let rpc: Rpc;
    let payer: Signer;
    let mint: PublicKey;
    let mintAuthority: Keypair;

    beforeAll(async () => {
        const lightWasm = await WasmFactory.getInstance();
        rpc = await getTestRpc(lightWasm);
        payer = await newAccountWithLamports(rpc, 1e9);
    });

    it('should create mint', async () => {
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
        const poolAccount = CompressedTokenProgram.deriveTokenPoolPda(mint);

        assert(mint.equals(mintKeypair.publicKey));

        await assertCreateMint(
            mint,
            mintAuthority.publicKey,
            rpc,
            TEST_TOKEN_DECIMALS,
            poolAccount,
        );

        /// Mint already exists
        await expect(
            createMint(
                rpc,
                payer,
                mintAuthority.publicKey,
                TEST_TOKEN_DECIMALS,
                mintKeypair,
            ),
        ).rejects.toThrow();
    });

    it('should create mint with payer as authority', async () => {
        mint = (
            await createMint(rpc, payer, payer.publicKey, TEST_TOKEN_DECIMALS)
        ).mint;

        const poolAccount = CompressedTokenProgram.deriveTokenPoolPda(mint);

        await assertCreateMint(
            mint,
            payer.publicKey,
            rpc,
            TEST_TOKEN_DECIMALS,
            poolAccount,
        );
    });
});
