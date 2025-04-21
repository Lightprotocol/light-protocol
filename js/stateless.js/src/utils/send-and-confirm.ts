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
import { isLocalTest } from '../constants';

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

    await confirmTx(rpc, txId, confirmOptions, blockHashCtx);

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
    _blockHashCtx?: { blockhash: string; lastValidBlockHeight: number }, // TODO: add this back in.
): Promise<RpcResponseAndContext<SignatureResult>> {
    const commitment = confirmOptions?.commitment || 'confirmed';
    const isLocal = isLocalTest(rpc.rpcEndpoint);

    const configs = [
        // local
        {
            local: true,
            commitment: 'confirmed',
            timeout: 15_000,
            interval: 200,
        },
        {
            local: true,
            commitment: 'finalized',
            timeout: 80_000,
            interval: 1000,
        },
        // devnet, mainnet
        {
            local: false,
            commitment: 'confirmed',
            timeout: 35_000,
            interval: 1000,
        },
        {
            local: false,
            commitment: 'finalized',
            timeout: 100_000,
            interval: 1000,
        },
    ];

    const config = configs.find(
        c => c.local === isLocal && c.commitment === commitment,
    );

    if (!config) {
        throw new Error(
            'No config found for local: ' +
                isLocal +
                ' and commitment: ' +
                commitment,
        );
    }

    const { timeout, interval } = config;

    let elapsed = 0;

    const res = await new Promise<TransactionSignature>((resolve, reject) => {
        const intervalId = setInterval(async () => {
            elapsed += interval;

            if (elapsed >= timeout) {
                clearInterval(intervalId);
                reject(
                    new Error(`Transaction ${txId}'s confirmation timed out`),
                );
            }

            const status = await rpc.getSignatureStatuses([txId]);

            if (status?.value[0]?.confirmationStatus === commitment) {
                clearInterval(intervalId);
                resolve(txId);
            }
        }, interval);
    });

    const slot = await rpc.getSlot();
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
