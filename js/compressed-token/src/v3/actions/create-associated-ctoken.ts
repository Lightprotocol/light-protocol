import {
    ComputeBudgetProgram,
    ConfirmOptions,
    PublicKey,
    Signer,
} from '@solana/web3.js';
import {
    Rpc,
    buildAndSignTx,
    sendAndConfirmTx,
    assertBetaEnabled,
} from '@lightprotocol/stateless.js';
import {
    createAssociatedCTokenAccountInstruction,
    createAssociatedCTokenAccountIdempotentInstruction,
    CompressibleConfig,
} from '../instructions/create-associated-ctoken';
import { getAssociatedCTokenAddress } from '../derivation';

/**
 * Create an associated c-token account.
 *
 * @param rpc                RPC connection
 * @param payer              Fee payer
 * @param owner              Owner of the associated token account
 * @param mint               Mint address
 * @param compressibleConfig Optional compressible configuration
 * @param configAccount      Optional config account
 * @param rentPayerPda       Optional rent payer PDA
 * @param confirmOptions     Optional confirm options
 * @returns Address of the new associated token account
 */
export async function createAssociatedCTokenAccount(
    rpc: Rpc,
    payer: Signer,
    owner: PublicKey,
    mint: PublicKey,
    compressibleConfig?: CompressibleConfig,
    configAccount?: PublicKey,
    rentPayerPda?: PublicKey,
    confirmOptions?: ConfirmOptions,
): Promise<PublicKey> {
    assertBetaEnabled();

    const ix = createAssociatedCTokenAccountInstruction(
        payer.publicKey,
        owner,
        mint,
        compressibleConfig,
        configAccount,
        rentPayerPda,
    );

    const { blockhash } = await rpc.getLatestBlockhash();
    const tx = buildAndSignTx(
        [ComputeBudgetProgram.setComputeUnitLimit({ units: 200_000 }), ix],
        payer,
        blockhash,
        [],
    );

    await sendAndConfirmTx(rpc, tx, confirmOptions);

    return getAssociatedCTokenAddress(owner, mint);
}

/**
 * Create an associated compressed token account idempotently.
 *
 * @param rpc                RPC connection
 * @param payer              Fee payer
 * @param owner              Owner of the associated token account
 * @param mint               Mint address
 * @param compressibleConfig Optional compressible configuration
 * @param configAccount      Optional config account
 * @param rentPayerPda       Optional rent payer PDA
 * @param confirmOptions     Optional confirm options
 * @returns Address of the associated token account
 */
export async function createAssociatedCTokenAccountIdempotent(
    rpc: Rpc,
    payer: Signer,
    owner: PublicKey,
    mint: PublicKey,
    compressibleConfig?: CompressibleConfig,
    configAccount?: PublicKey,
    rentPayerPda?: PublicKey,
    confirmOptions?: ConfirmOptions,
): Promise<PublicKey> {
    assertBetaEnabled();

    const ix = createAssociatedCTokenAccountIdempotentInstruction(
        payer.publicKey,
        owner,
        mint,
        compressibleConfig,
        configAccount,
        rentPayerPda,
    );

    const { blockhash } = await rpc.getLatestBlockhash();
    const tx = buildAndSignTx(
        [ComputeBudgetProgram.setComputeUnitLimit({ units: 200_000 }), ix],
        payer,
        blockhash,
        [],
    );

    await sendAndConfirmTx(rpc, tx, confirmOptions);

    return getAssociatedCTokenAddress(owner, mint);
}
