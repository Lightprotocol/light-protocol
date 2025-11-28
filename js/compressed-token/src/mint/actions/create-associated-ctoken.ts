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
    const ix = createAssociatedCTokenAccountInstruction({
        feePayer: payer.publicKey,
        owner,
        mint,
        compressibleConfig,
        configAccount,
        rentPayerPda,
    });

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
    const ix = createAssociatedCTokenAccountIdempotentInstruction({
        feePayer: payer.publicKey,
        owner,
        mint,
        compressibleConfig,
        configAccount,
        rentPayerPda,
    });

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
