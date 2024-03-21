import {
    Connection,
    VersionedTransaction,
    TransactionConfirmationStrategy,
    SignatureResult,
    RpcResponseAndContext,
    Signer,
    TransactionInstruction,
    TransactionMessage,
    ConfirmOptions,
    TransactionSignature,
} from '@solana/web3.js';

/** Sends a versioned transaction and confirms it. */
export async function sendAndConfirmTx(
    connection: Connection,
    tx: VersionedTransaction,
    confirmOptions?: ConfirmOptions,
): Promise<TransactionSignature> {
    const txId = await connection.sendTransaction(tx, confirmOptions);
    const { blockhash, lastValidBlockHeight } =
        await connection.getLatestBlockhash(confirmOptions?.commitment);
    const transactionConfirmationStrategy0: TransactionConfirmationStrategy = {
        signature: txId,
        blockhash,
        lastValidBlockHeight,
    };
    await connection.confirmTransaction(
        transactionConfirmationStrategy0,
        confirmOptions?.commitment || connection.commitment || 'confirmed',
    );
    return txId;
}

/** @internal */
export async function confirmTx(
    connection: Connection,
    txId: string,
    blockHashCtx?: { blockhash: string; lastValidBlockHeight: number },
): Promise<RpcResponseAndContext<SignatureResult>> {
    if (!blockHashCtx) blockHashCtx = await connection.getLatestBlockhash();

    const transactionConfirmationStrategy: TransactionConfirmationStrategy = {
        signature: txId,
        blockhash: blockHashCtx.blockhash,
        lastValidBlockHeight: blockHashCtx.lastValidBlockHeight,
    };
    const res = await connection.confirmTransaction(
        transactionConfirmationStrategy,
        connection.commitment || 'confirmed',
    );
    return res;
}

/**
 * Builds a versioned Transaction from instructions and signs it.
 *
 * @param instructions        instructions to include in the transaction
 * @param payer               payer of the transaction
 * @param blockhash           recent blockhash to use in the transaction
 * @param additionalSigners   non-feepayer signers to include in the transaction
 */
export function buildAndSignTx(
    instructions: TransactionInstruction[],
    payer: Signer,
    blockhash: string,
    additionalSigners: Signer[] = [],
): VersionedTransaction {
    if (additionalSigners.includes(payer))
        throw new Error('payer must not be in additionalSigners');
    const allSigners = [payer, ...additionalSigners];

    const messageV0 = new TransactionMessage({
        payerKey: payer.publicKey,
        recentBlockhash: blockhash,
        instructions,
    }).compileToV0Message();

    const tx = new VersionedTransaction(messageV0);
    tx.sign(allSigners);
    return tx;
}
