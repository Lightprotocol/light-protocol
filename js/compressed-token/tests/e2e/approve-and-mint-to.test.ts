import { describe, it, expect, beforeAll, assert } from 'vitest';
import { PublicKey, Signer, Keypair, SystemProgram } from '@solana/web3.js';
import {
    MINT_SIZE,
    TOKEN_2022_PROGRAM_ID,
    TOKEN_PROGRAM_ID,
    createInitializeMint2Instruction,
} from '@solana/spl-token';
import { approveAndMintTo, createTokenPool } from '../../src/actions';
import {
    Rpc,
    bn,
    buildAndSignTx,
    dedupeSigner,
    newAccountWithLamports,
    sendAndConfirmTx,
    getTestRpc,
    defaultTestStateTreeAccounts,
    TreeInfo,
    selectStateTreeInfo,
} from '@lightprotocol/stateless.js';
import { WasmFactory } from '@lightprotocol/hasher.rs';
import BN from 'bn.js';
import {
    getTokenPoolInfos,
    selectTokenPoolInfo,
    TokenPoolInfo,
} from '../../src/utils/get-token-pool-infos';

async function createTestSplMint(
    rpc: Rpc,
    payer: Signer,
    mintKeypair: Signer,
    mintAuthority: Keypair,
    isToken2022 = false,
) {
    const programId = isToken2022 ? TOKEN_2022_PROGRAM_ID : TOKEN_PROGRAM_ID;
    const rentExemptBalance =
        await rpc.getMinimumBalanceForRentExemption(MINT_SIZE);

    const createMintAccountInstruction = SystemProgram.createAccount({
        fromPubkey: payer.publicKey,
        lamports: rentExemptBalance,
        newAccountPubkey: mintKeypair.publicKey,
        programId,
        space: MINT_SIZE,
    });
    const initializeMintInstruction = createInitializeMint2Instruction(
        mintKeypair.publicKey,
        TEST_TOKEN_DECIMALS,
        mintAuthority.publicKey,
        null,
        programId,
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
describe('approveAndMintTo', () => {
    let rpc: Rpc;
    let payer: Signer;
    let bob: PublicKey;
    let mintKeypair: Keypair;
    let mint: PublicKey;
    let mintAuthority: Keypair;
    let tokenPoolInfo: TokenPoolInfo;
    let stateTreeInfo: TreeInfo;

    beforeAll(async () => {
        const lightWasm = await WasmFactory.getInstance();
        rpc = await getTestRpc(lightWasm);
        payer = await newAccountWithLamports(rpc);
        bob = Keypair.generate().publicKey;
        mintAuthority = Keypair.generate();
        mintKeypair = Keypair.generate();
        mint = mintKeypair.publicKey;

        /// Create external SPL mint
        await createTestSplMint(rpc, payer, mintKeypair, mintAuthority);

        /// Register mint
        await createTokenPool(rpc, payer, mint);
        tokenPoolInfo = selectTokenPoolInfo(await getTokenPoolInfos(rpc, mint));
        stateTreeInfo = selectStateTreeInfo(await rpc.getStateTreeInfos());
    });

    it('should mintTo compressed account with external spl mint', async () => {
        assert(mint.equals(mintKeypair.publicKey));

        await approveAndMintTo(
            rpc,
            payer,
            mint,
            bob,
            mintAuthority,
            1000000000,
            stateTreeInfo,
            tokenPoolInfo,
        );

        await assertApproveAndMintTo(rpc, mint, bn(1000000000), bob);
    });

    it('should mintTo compressed account with external token 2022 mint', async () => {
        const payer = await newAccountWithLamports(rpc);
        const bob = Keypair.generate().publicKey;
        const token22MintAuthority = Keypair.generate();
        const token22MintKeypair = Keypair.generate();
        const token22Mint = token22MintKeypair.publicKey;

        /// Create external SPL mint
        await createTestSplMint(
            rpc,
            payer,
            token22MintKeypair,
            token22MintAuthority,
            true,
        );
        const mintAccountInfo = await rpc.getAccountInfo(token22Mint);
        assert(mintAccountInfo!.owner.equals(TOKEN_2022_PROGRAM_ID));
        /// Register mint
        await createTokenPool(rpc, payer, token22Mint);
        assert(token22Mint.equals(token22MintKeypair.publicKey));

        const tokenPoolInfoT22 = selectTokenPoolInfo(
            await getTokenPoolInfos(rpc, token22Mint),
        );

        await approveAndMintTo(
            rpc,
            payer,
            token22Mint,
            bob,
            token22MintAuthority,
            1000000000,
            stateTreeInfo,
            tokenPoolInfoT22,
        );

        await assertApproveAndMintTo(rpc, token22Mint, bn(1000000000), bob);
    });
});

/**
 * Assert that approveAndMintTo() creates a new compressed token account for the
 * recipient
 */
async function assertApproveAndMintTo(
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

    const compressedTokenAccount = compressedTokenAccounts.items[0];
    expect(compressedTokenAccount.parsed.mint.toBase58()).toBe(
        refMint.toBase58(),
    );
    expect(compressedTokenAccount.parsed.amount.eq(refAmount)).toBe(true);
    expect(compressedTokenAccount.parsed.owner.equals(refTo)).toBe(true);
    expect(compressedTokenAccount.parsed.delegate).toBe(null);
}
