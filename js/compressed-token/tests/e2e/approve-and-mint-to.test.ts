import { describe, it, expect, beforeAll, assert } from 'vitest';
import { PublicKey, Signer, Keypair, SystemProgram } from '@solana/web3.js';
import {
    MINT_SIZE,
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
} from '@lightprotocol/stateless.js';
import { WasmFactory } from '@lightprotocol/hasher.rs';
import { BN } from '@coral-xyz/anchor';

async function createTestSplMint(
    rpc: Rpc,
    payer: Signer,
    mintKeypair: Signer,
    mintAuthority: Keypair,
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
        TOKEN_PROGRAM_ID,
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
        );

        await assertApproveAndMintTo(rpc, mint, bn(1000000000), bob);
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

    const compressedTokenAccount = compressedTokenAccounts[0];
    expect(compressedTokenAccount.parsed.mint.toBase58()).toBe(
        refMint.toBase58(),
    );
    expect(compressedTokenAccount.parsed.amount.eq(refAmount)).toBe(true);
    expect(compressedTokenAccount.parsed.owner.equals(refTo)).toBe(true);
    expect(compressedTokenAccount.parsed.delegate).toBe(null);
}
