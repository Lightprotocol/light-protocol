import { describe, it, expect, beforeAll, assert } from 'vitest';
import { CompressedTokenProgram } from '../../src/program';
import { PublicKey, Signer, Keypair, SystemProgram } from '@solana/web3.js';
import {
    unpackMint,
    unpackAccount,
    MINT_SIZE,
    TOKEN_PROGRAM_ID,
    createInitializeMint2Instruction,
} from '@solana/spl-token';
import { createMint, createTokenPool } from '../../src/actions';
import {
    Rpc,
    buildAndSignTx,
    dedupeSigner,
    newAccountWithLamports,
    sendAndConfirmTx,
    getTestRpc,
    StateTreeInfo,
} from '@lightprotocol/stateless.js';
import { WasmFactory } from '@lightprotocol/hasher.rs';
import { TOKEN_2022_PROGRAM_ID } from '@solana/spl-token';

/**
 * Assert that createTokenPool() creates system-pool account for external mint,
 * with external mintAuthority.
 */
async function assertRegisterMint(
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

async function createTestSplMint(
    rpc: Rpc,
    payer: Signer,
    mintKeypair: Signer,
    mintAuthority: Keypair,
    isToken22?: boolean,
) {
    const rentExemptBalance =
        await rpc.getMinimumBalanceForRentExemption(MINT_SIZE);

    const createMintAccountInstruction = SystemProgram.createAccount({
        fromPubkey: payer.publicKey,
        lamports: rentExemptBalance,
        newAccountPubkey: mintKeypair.publicKey,
        programId: TOKEN_PROGRAM_ID,
        space: MINT_SIZE,
    });
    const initializeMintInstruction = createInitializeMint2Instruction(
        mintKeypair.publicKey,
        TEST_TOKEN_DECIMALS,
        mintAuthority.publicKey,
        null,
        isToken22 ? TOKEN_2022_PROGRAM_ID : TOKEN_PROGRAM_ID,
    );
    const { blockhash } = await rpc.getLatestBlockhash();

    const tx = buildAndSignTx(
        [createMintAccountInstruction, initializeMintInstruction],
        payer,
        blockhash,
        dedupeSigner(payer, [mintKeypair]),
    );
    await sendAndConfirmTx(rpc, tx);
}

const TEST_TOKEN_DECIMALS = 2;

describe('createTokenPool', () => {
    let rpc: Rpc;
    let payer: Signer;
    let mintKeypair: Keypair;
    let mint: PublicKey;
    let mintAuthority: Keypair;
    let outputStateTreeInfo: StateTreeInfo;

    beforeAll(async () => {
        const lightWasm = await WasmFactory.getInstance();
        rpc = await getTestRpc(lightWasm);
        outputStateTreeInfo = (await rpc.getCachedActiveStateTreeInfos())[0];
        payer = await newAccountWithLamports(rpc);
        mintAuthority = Keypair.generate();
        mintKeypair = Keypair.generate();
        mint = mintKeypair.publicKey;

        /// Create external SPL mint
        await createTestSplMint(rpc, payer, mintKeypair, mintAuthority);
    });

    it('should register existing spl mint', async () => {
        const poolAccount = CompressedTokenProgram.deriveTokenPoolPda(mint);

        assert(mint.equals(mintKeypair.publicKey));

        /// Mint already exists externally
        await expect(
            createMint(
                rpc,
                payer,
                mintAuthority.publicKey,
                TEST_TOKEN_DECIMALS,
                mintKeypair,
            ),
        ).rejects.toThrow();

        await createTokenPool(rpc, payer, mint);

        await assertRegisterMint(
            mint,
            mintAuthority.publicKey,
            rpc,
            TEST_TOKEN_DECIMALS,
            poolAccount,
        );

        /// Mint already registered
        await expect(createTokenPool(rpc, payer, mint)).rejects.toThrow();
    });
    it('should register existing spl token22 mint', async () => {
        const token22MintKeypair = Keypair.generate();
        const token22Mint = token22MintKeypair.publicKey;
        const token22MintAuthority = Keypair.generate();

        /// Create external SPL Token 2022 mint
        await createTestSplMint(
            rpc,
            payer,
            token22MintKeypair,
            token22MintAuthority,
        );

        const poolAccount =
            CompressedTokenProgram.deriveTokenPoolPda(token22Mint);

        assert(token22Mint.equals(token22MintKeypair.publicKey));

        /// Mint already exists externally
        await expect(
            createMint(
                rpc,
                payer,
                token22MintAuthority.publicKey,
                TEST_TOKEN_DECIMALS,
                token22MintKeypair,
                undefined,
                true,
            ),
        ).rejects.toThrow();

        await createTokenPool(rpc, payer, token22Mint);

        await assertRegisterMint(
            token22Mint,
            token22MintAuthority.publicKey,
            rpc,
            TEST_TOKEN_DECIMALS,
            poolAccount,
        );

        /// Mint already registered
        await expect(
            createTokenPool(rpc, payer, token22Mint),
        ).rejects.toThrow();
    });

    it('should create mint with payer as authority', async () => {
        /// Create new external SPL mint with payer === authority
        mintKeypair = Keypair.generate();
        mint = mintKeypair.publicKey;
        await createTestSplMint(rpc, payer, mintKeypair, payer as Keypair);
        await createTokenPool(rpc, payer, mint);

        const poolAccount = CompressedTokenProgram.deriveTokenPoolPda(mint);
        await assertRegisterMint(
            mint,
            payer.publicKey,
            rpc,
            TEST_TOKEN_DECIMALS,
            poolAccount,
        );
    });
});
