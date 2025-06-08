import { describe, it, expect, beforeAll, assert } from 'vitest';
import { CompressedTokenProgram } from '../../src/program';
import { PublicKey, Signer, Keypair, SystemProgram } from '@solana/web3.js';
import {
    MINT_SIZE,
    TOKEN_PROGRAM_ID,
    createInitializeMint2Instruction,
    getOrCreateAssociatedTokenAccount,
    mintTo,
} from '@solana/spl-token';
import {
    addTokenPools,
    compress,
    createMint,
    createTokenPool,
    decompress,
} from '../../src/actions';
import {
    Rpc,
    buildAndSignTx,
    dedupeSigner,
    newAccountWithLamports,
    sendAndConfirmTx,
    getTestRpc,
    selectStateTreeInfo,
} from '@lightprotocol/stateless.js';
import { WasmFactory } from '@lightprotocol/hasher.rs';
import { TOKEN_2022_PROGRAM_ID } from '@solana/spl-token';
import {
    getTokenPoolInfos,
    selectTokenPoolInfo,
    selectTokenPoolInfosForDecompression,
} from '../../src/utils';

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
        programId: isToken22 ? TOKEN_2022_PROGRAM_ID : TOKEN_PROGRAM_ID,
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
describe('multi-pool', () => {
    let rpc: Rpc;
    let payer: Signer;
    let mintKeypair: Keypair;
    let mint: PublicKey;
    let mintAuthority: Keypair;

    let bob: Signer;
    let bobAta: PublicKey;
    let charlie: Signer;
    let charlieAta: PublicKey;

    beforeAll(async () => {
        const lightWasm = await WasmFactory.getInstance();
        rpc = await getTestRpc(lightWasm);
        payer = await newAccountWithLamports(rpc);
        mintAuthority = Keypair.generate();
        mintKeypair = Keypair.generate();
        mint = mintKeypair.publicKey;

        /// Create external SPL mint
        await createTestSplMint(rpc, payer, mintKeypair, mintAuthority);

        bob = await newAccountWithLamports(rpc);
        bobAta = (
            await getOrCreateAssociatedTokenAccount(
                rpc,
                payer,
                mint,
                bob.publicKey,
            )
        ).address;
        charlie = await newAccountWithLamports(rpc);
        charlieAta = (
            await getOrCreateAssociatedTokenAccount(
                rpc,
                payer,
                mint,
                charlie.publicKey,
            )
        ).address;
        await mintTo(rpc, payer, mint, bobAta, mintAuthority, BigInt(1000));
    });

    it('should register 4 pools', async () => {
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
        await addTokenPools(rpc, payer, mint, 3);

        const stateTreeInfos = await rpc.getStateTreeInfos();
        const info = selectStateTreeInfo(stateTreeInfos);

        const tokenPoolInfos = await getTokenPoolInfos(rpc, mint);

        expect(tokenPoolInfos.length).toBe(5);
        expect(tokenPoolInfos[4].poolIndex).toBe(4);
        expect(tokenPoolInfos[4].isInitialized).toBe(false);
        const tokenPoolInfo = selectTokenPoolInfo(tokenPoolInfos);

        await compress(
            rpc,
            payer,
            mint,
            100,
            bob,
            bobAta,
            charlie.publicKey,
            info,
            tokenPoolInfo,
        );

        const tokenPoolInfos2 = await getTokenPoolInfos(rpc, mint);
        const tokenPoolInfosForDecompression =
            selectTokenPoolInfosForDecompression(tokenPoolInfos2, 10);

        await decompress(
            rpc,
            payer,
            mint,
            10,
            charlie,
            charlieAta,
            tokenPoolInfosForDecompression,
        );

        const tokenPoolInfos3 = await getTokenPoolInfos(rpc, mint);
        const tokenPoolInfosForDecompression3 =
            selectTokenPoolInfosForDecompression(tokenPoolInfos3, 90);

        expect(tokenPoolInfosForDecompression3.length).toBe(4);

        await decompress(
            rpc,
            payer,
            mint,
            90,
            charlie,
            charlieAta,
            tokenPoolInfosForDecompression3,
        );

        const tokenPoolInfos4 = await getTokenPoolInfos(rpc, mint);
        expect(tokenPoolInfos4.length).toBe(5);
        expect(() => {
            selectTokenPoolInfosForDecompression(tokenPoolInfos4, 1);
        }).toThrowError(
            'All provided token pool balances are zero. Please pass recent token pool infos.',
        );
    });
});
