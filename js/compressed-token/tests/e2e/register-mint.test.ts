import { describe, it, expect, beforeAll, assert, beforeEach } from 'vitest';
import { CompressedTokenProgram } from '../../src/program';
import { PublicKey, Signer, Keypair, SystemProgram } from '@solana/web3.js';
import {
    unpackMint,
    unpackAccount,
    MINT_SIZE,
    TOKEN_PROGRAM_ID,
    createInitializeMint2Instruction,
} from '@solana/spl-token';
import { createMint, registerMint } from '../../src/actions';
import {
    Rpc,
    buildAndSignTx,
    createRpc,
    dedupeSigner,
    newAccountWithLamports,
    sendAndConfirmTx,
} from '@lightprotocol/stateless.js';

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

async function createTestSplMint(
    rpc: Rpc,
    payer: Signer,
    mint: PublicKey,
    mintAuthority: Keypair,
) {
    const rentExemptBalance =
        await rpc.getMinimumBalanceForRentExemption(MINT_SIZE);

    const createMintAccountInstruction = SystemProgram.createAccount({
        fromPubkey: payer.publicKey,
        lamports: rentExemptBalance,
        newAccountPubkey: mint,
        programId: TOKEN_PROGRAM_ID,
        space: MINT_SIZE,
    });
    const initializeMintInstruction = createInitializeMint2Instruction(
        mint,
        TEST_TOKEN_DECIMALS,
        mintAuthority.publicKey,
        null,
        TOKEN_PROGRAM_ID,
    );
    const { blockhash } = await rpc.getLatestBlockhash();
    const tx = buildAndSignTx(
        [createMintAccountInstruction, initializeMintInstruction],
        payer,
        blockhash,
        dedupeSigner(payer, [mintAuthority]),
    );
    await sendAndConfirmTx(rpc, tx);
}

const TEST_TOKEN_DECIMALS = 2;
describe('registerMint', () => {
    let rpc: Rpc;
    let payer: Signer;
    let mintKeypair: Keypair;
    let mint: PublicKey;
    let mintAuthority: Keypair;

    beforeAll(async () => {
        rpc = createRpc();
        payer = await newAccountWithLamports(rpc);
        mintAuthority = Keypair.generate();
        mintKeypair = Keypair.generate();

        /// Create external SPL mint
        await createTestSplMint(rpc, payer, mint, mintAuthority);
    });

    it('should register existing spl mint', async () => {
        const poolAccount = CompressedTokenProgram.deriveTokenPoolPda(mint);

        assert(mint.equals(mintKeypair.publicKey));

        /// Mint already exists externally
        await expect(
            createMint(
                rpc,
                payer,
                mintAuthority,
                TEST_TOKEN_DECIMALS,
                mintKeypair,
            ),
        ).rejects.toThrow();

        await registerMint(
            rpc,
            payer,
            mintAuthority,
            TEST_TOKEN_DECIMALS,
            mintKeypair,
        );

        await assertCreateMint(
            mint,
            mintAuthority.publicKey,
            rpc,
            TEST_TOKEN_DECIMALS,
            poolAccount,
        );

        /// Mint already registered
        await expect(
            registerMint(
                rpc,
                payer,
                mintAuthority,
                TEST_TOKEN_DECIMALS,
                mintKeypair,
            ),
        ).rejects.toThrow();
    });

    it('should create mint with payer as authority', async () => {
        /// Create new external SPL mint with payer === authority
        mintKeypair = Keypair.generate();
        await createTestSplMint(rpc, payer, mint, payer as Keypair);
        await registerMint(rpc, payer, payer, TEST_TOKEN_DECIMALS, mintKeypair);

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
