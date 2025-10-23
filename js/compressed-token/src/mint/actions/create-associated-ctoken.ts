import {
    ComputeBudgetProgram,
    ConfirmOptions,
    PublicKey,
    Signer,
    TransactionSignature,
} from '@solana/web3.js';
import {
    Rpc,
    buildAndSignTx,
    sendAndConfirmTx,
} from '@lightprotocol/stateless.js';
import {
    createAssociatedCTokenAccountInstruction,
    createAssociatedCTokenAccountIdempotentInstruction,
    CompressibleConfig,
} from '../instructions/create-associated-ctoken';
import { getAssociatedCTokenAddress } from '../../compressible';

/**
 * Create an associated compressed token account.
 *
 * @param rpc                RPC connection
 * @param payer              Fee payer
 * @param owner              Owner of the associated token account
 * @param mint               Mint address
 * @param compressibleConfig Optional compressible configuration
 * @param configAccount      Optional config account
 * @param rentPayerPda       Optional rent payer PDA
 * @param confirmOptions     Optional confirm options
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
): Promise<{ address: PublicKey; transactionSignature: TransactionSignature }> {
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

    const txId = await sendAndConfirmTx(rpc, tx, confirmOptions);
    const address = getAssociatedCTokenAddress(owner, mint);

    return { address, transactionSignature: txId };
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
): Promise<{ address: PublicKey; transactionSignature: TransactionSignature }> {
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

    const txId = await sendAndConfirmTx(rpc, tx, confirmOptions);
    const address = getAssociatedCTokenAddress(owner, mint);

    return { address, transactionSignature: txId };
}
