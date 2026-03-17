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
    createAssociatedLightTokenAccountInstruction,
    createAssociatedLightTokenAccountIdempotentInstruction,
    CompressibleConfig,
} from '../instructions/create-associated-light-token';
import { getAssociatedLightTokenAddress } from '../derivation';

/**
 * Create an associated light-token account.
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
 * @internal
 */
export async function createAssociatedLightTokenAccount(
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

    const ix = createAssociatedLightTokenAccountInstruction(
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

    return getAssociatedLightTokenAddress(owner, mint);
}

/**
 * Create an associated light-token account idempotently.
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
 * @internal
 */
export async function createAssociatedLightTokenAccountIdempotent(
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

    const ix = createAssociatedLightTokenAccountIdempotentInstruction(
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

    return getAssociatedLightTokenAddress(owner, mint);
}
