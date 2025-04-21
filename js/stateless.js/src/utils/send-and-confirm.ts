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
    AddressLookupTableAccount,
    SignatureStatus,
    SignatureStatusConfig,
} from '@solana/web3.js';
import { Rpc } from '../rpc';
import { sleep } from './sleep';

/**
 * Builds a versioned Transaction from instructions.
 *
 * @param instructions          instructions to include
 * @param payerPublicKey        fee payer public key
 * @param blockhash             blockhash to use
 * @param lookupTableAccounts   lookup table accounts to include
 *
 * @return VersionedTransaction
 */
export function buildTx(
    instructions: TransactionInstruction[],
    payerPublicKey: PublicKey,
    blockhash: string,
    lookupTableAccounts?: AddressLookupTableAccount[],
): VersionedTransaction {
    const messageV0 = new TransactionMessage({
        payerKey: payerPublicKey,
        recentBlockhash: blockhash,
        instructions,
    }).compileToV0Message(lookupTableAccounts);

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

    const ctxAndRes = await confirmTx(rpc, txId, confirmOptions, blockHashCtx);
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

    const commitment =
        confirmOptions?.commitment || rpc.commitment || 'confirmed';
    let status: SignatureStatus | null = null;
    let slot: number = 0;

    const signatureStatusConfig: SignatureStatusConfig = {
        searchTransactionHistory: false,
    };

    while (!status || status.confirmationStatus !== commitment) {
        const res = await rpc.getSignatureStatuses(
            [txId],
            signatureStatusConfig,
        );
        status = res.value[0];
        slot = res.context.slot;

        if (!status) {
            throw new Error('Transaction not found');
        }

        if (status.err) {
            throw new Error(
                `Transaction failed: ${JSON.stringify(status.err)}`,
            );
        }

        const currentBlockHeight = await rpc.getBlockHeight();
        if (
            blockHashCtx &&
            currentBlockHeight > blockHashCtx.lastValidBlockHeight
        ) {
            throw new Error('Transaction expired: block height exceeded');
        }

        await sleep(400);
    }

    await rpc.confirmTransactionIndexed(slot);
    return { context: { slot }, value: { err: null } };
}

/**
 * Builds a versioned Transaction from instructions and signs it.
 *
 * @param instructions          instructions to include in the transaction
 * @param payer                 payer of the transaction
 * @param blockhash             recent blockhash to use in the transaction
 * @param additionalSigners     non-feepayer signers to include in the
 *                              transaction
 * @param lookupTableAccounts   lookup table accounts to include in the
 *                              transaction
 */
export function buildAndSignTx(
    instructions: TransactionInstruction[],
    payer: Signer,
    blockhash: string,
    additionalSigners: Signer[] = [],
    lookupTableAccounts?: AddressLookupTableAccount[],
): VersionedTransaction {
    if (additionalSigners.includes(payer))
        throw new Error('payer must not be in additionalSigners');
    const allSigners = [payer, ...additionalSigners];

    const tx = buildTx(
        instructions,
        payer.publicKey,
        blockhash,
        lookupTableAccounts,
    );

    tx.sign(allSigners);

    return tx;
}
