import {
    VersionedTransaction,
    TransactionConfirmationStrategy,
    SignatureResult,
    RpcResponseAndContext,
    Signer,
    TransactionInstruction,
    TransactionMessage,
    ConfirmOptions,
    TransactionSignature,
    PublicKey,
} from '@solana/web3.js';
import { Rpc } from '../rpc';

/**
 * Builds a versioned Transaction from instructions.
 *
 * @param instructions        instructions to include
 * @param payerPublicKey      fee payer public key
 * @param blockhash          blockhash to use
 *
 * @return VersionedTransaction
 */
export function buildTx(
    instructions: TransactionInstruction[],
    payerPublicKey: PublicKey,
    blockhash: string,
): VersionedTransaction {
    const messageV0 = new TransactionMessage({
        payerKey: payerPublicKey,
        recentBlockhash: blockhash,
        instructions,
    }).compileToV0Message();

    return new VersionedTransaction(messageV0);
}

/**
 * Sends a versioned transaction and confirms it.
 *
 * @param rpc               connection to use
 * @param tx                versioned transaction to send
 * @param confirmOptions    confirmation options
 * @param blockHashCtx      blockhash context for confirmation
 *
 * @return TransactionSignature
 */
export async function sendAndConfirmTx(
    rpc: Rpc,
    tx: VersionedTransaction,
    confirmOptions?: ConfirmOptions,
    blockHashCtx?: { blockhash: string; lastValidBlockHeight: number },
): Promise<TransactionSignature> {
    const txId = await rpc.sendTransaction(tx, confirmOptions);

    if (!blockHashCtx) blockHashCtx = await rpc.getLatestBlockhash();

    const transactionConfirmationStrategy0: TransactionConfirmationStrategy = {
        signature: txId,
        blockhash: blockHashCtx.blockhash,
        lastValidBlockHeight: blockHashCtx.lastValidBlockHeight,
    };

    const ctxAndRes = await rpc.confirmTransaction(
        transactionConfirmationStrategy0,
        confirmOptions?.commitment || rpc.commitment || 'confirmed',
    );
    const slot = ctxAndRes.context.slot;
    await rpc.confirmTransactionIndexed(slot);
    return txId;
}

/**
 * Confirms a transaction with a given txId.
 *
 * @param rpc               connection to use
 * @param txId              transaction signature to confirm
 * @param confirmOptions    confirmation options
 * @param blockHashCtx      blockhash context for confirmation
 * @return SignatureResult
 */
export async function confirmTx(
    rpc: Rpc,
    txId: string,
    confirmOptions?: ConfirmOptions,
    blockHashCtx?: { blockhash: string; lastValidBlockHeight: number },
): Promise<RpcResponseAndContext<SignatureResult>> {
    if (!blockHashCtx) blockHashCtx = await rpc.getLatestBlockhash();

    const transactionConfirmationStrategy: TransactionConfirmationStrategy = {
        signature: txId,
        blockhash: blockHashCtx.blockhash,
        lastValidBlockHeight: blockHashCtx.lastValidBlockHeight,
    };
    const res = await rpc.confirmTransaction(
        transactionConfirmationStrategy,
        confirmOptions?.commitment || rpc.commitment || 'confirmed',
    );
    const slot = res.context.slot;
    await rpc.confirmTransactionIndexed(slot);
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

    const tx = buildTx(instructions, payer.publicKey, blockhash);

    tx.sign(allSigners);

    return tx;
}
