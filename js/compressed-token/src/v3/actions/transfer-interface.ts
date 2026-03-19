import {
    ConfirmOptions,
    ComputeBudgetProgram,
    PublicKey,
    Signer,
    TransactionInstruction,
    TransactionSignature,
} from '@solana/web3.js';
import {
    Rpc,
    buildAndSignTx,
    sendAndConfirmTx,
    dedupeSigner,
    assertBetaEnabled,
    LIGHT_TOKEN_PROGRAM_ID,
} from '@lightprotocol/stateless.js';
import BN from 'bn.js';
import { createTransferToAccountInterfaceInstructions } from '../instructions/transfer-interface';
import { getAssociatedTokenAddressInterface } from '../get-associated-token-address-interface';
import { getMintInterface } from '../get-mint-interface';
import { type SplInterfaceInfo } from '../../utils/get-token-pool-infos';
import { sliceLast } from './slice-last';
import { createAssociatedTokenAccountInterfaceIdempotentInstruction } from '../instructions/create-ata-interface';
import { assertTransactionSizeWithinLimit } from '../utils/estimate-tx-size';

export interface InterfaceOptions {
    splInterfaceInfos?: SplInterfaceInfo[];
    /**
     * ATA owner (authority owner) used to derive the ATA when the signer is a
     * delegate. For owner-signed flows, omit this field.
     */
    owner?: PublicKey;
}

export async function transferToAccountInterface(
    rpc: Rpc,
    payer: Signer,
    source: PublicKey,
    mint: PublicKey,
    destination: PublicKey,
    owner: Signer,
    amount: number | bigint | BN,
    programId: PublicKey = LIGHT_TOKEN_PROGRAM_ID,
    confirmOptions?: ConfirmOptions,
    options?: InterfaceOptions,
    wrap = false,
    decimals?: number,
): Promise<TransactionSignature> {
    assertBetaEnabled();

    const effectiveOwner = options?.owner ?? owner.publicKey;
    const expectedSource = getAssociatedTokenAddressInterface(
        mint,
        effectiveOwner,
        false,
        programId,
    );
    if (!source.equals(expectedSource)) {
        throw new Error(
            `Source mismatch. Expected ${expectedSource.toBase58()}, got ${source.toBase58()}`,
        );
    }

    const amountBigInt = BigInt(amount.toString());

    const resolvedDecimals =
        decimals ?? (await getMintInterface(rpc, mint)).mint.decimals;
    const batches = await createTransferToAccountInterfaceInstructions(
        rpc,
        payer.publicKey,
        mint,
        amountBigInt,
        owner.publicKey,
        destination,
        resolvedDecimals,
        {
            ...options,
            wrap,
            programId,
        },
    );

    const additionalSigners = dedupeSigner(payer, [owner]);
    const { rest: loads, last: transferIxs } = sliceLast(batches);
    await Promise.all(
        loads.map(async ixs => {
            const { blockhash } = await rpc.getLatestBlockhash();
            const tx = buildAndSignTx(ixs, payer, blockhash, additionalSigners);
            return sendAndConfirmTx(rpc, tx, confirmOptions);
        }),
    );
    const { blockhash } = await rpc.getLatestBlockhash();
    const tx = buildAndSignTx(transferIxs, payer, blockhash, additionalSigners);
    return sendAndConfirmTx(rpc, tx, confirmOptions);
}

export async function transferInterface(
    rpc: Rpc,
    payer: Signer,
    source: PublicKey,
    mint: PublicKey,
    recipient: PublicKey,
    owner: Signer,
    amount: number | bigint | BN,
    programId: PublicKey = LIGHT_TOKEN_PROGRAM_ID,
    confirmOptions?: ConfirmOptions,
    options?: InterfaceOptions,
    wrap = false,
    decimals?: number,
): Promise<TransactionSignature> {
    assertBetaEnabled();

    const effectiveOwner = options?.owner ?? owner.publicKey;
    const expectedSource = getAssociatedTokenAddressInterface(
        mint,
        effectiveOwner,
        false,
        programId,
    );
    if (!source.equals(expectedSource)) {
        throw new Error(
            `Source mismatch. Expected ${expectedSource.toBase58()}, got ${source.toBase58()}`,
        );
    }

    const resolvedDecimals =
        decimals ?? (await getMintInterface(rpc, mint)).mint.decimals;
    const batches = await createTransferInterfaceInstructions(
        rpc,
        payer.publicKey,
        mint,
        amount,
        owner.publicKey,
        recipient,
        resolvedDecimals,
        {
            ...options,
            wrap,
            programId,
        },
    );

    const additionalSigners = dedupeSigner(payer, [owner]);
    const { rest: loads, last: transferIxs } = sliceLast(batches);
    await Promise.all(
        loads.map(async ixs => {
            const { blockhash } = await rpc.getLatestBlockhash();
            const tx = buildAndSignTx(ixs, payer, blockhash, additionalSigners);
            return sendAndConfirmTx(rpc, tx, confirmOptions);
        }),
    );
    const { blockhash } = await rpc.getLatestBlockhash();
    const tx = buildAndSignTx(transferIxs, payer, blockhash, additionalSigners);
    return sendAndConfirmTx(rpc, tx, confirmOptions);
}

export interface TransferOptions extends InterfaceOptions {
    wrap?: boolean;
    programId?: PublicKey;
}
export type TransferToAccountOptions = TransferOptions;

export { sliceLast } from './slice-last';

export async function createTransferInterfaceInstructions(
    rpc: Rpc,
    payer: PublicKey,
    mint: PublicKey,
    amount: number | bigint | BN,
    sender: PublicKey,
    recipient: PublicKey,
    decimals: number,
    options?: TransferOptions,
): Promise<TransactionInstruction[][]> {
    // Convenience path intentionally derives ATA from a wallet recipient.
    // PDA/off-curve recipients should use transferToAccountInterface with an
    // explicitly derived destination token account.
    const programId = options?.programId ?? LIGHT_TOKEN_PROGRAM_ID;
    const destination = getAssociatedTokenAddressInterface(
        mint,
        recipient,
        false,
        programId,
    );
    const batches = await createTransferToAccountInterfaceInstructions(
        rpc,
        payer,
        mint,
        amount,
        sender,
        destination,
        decimals,
        options,
    );

    const ensureRecipientAtaIx =
        createAssociatedTokenAccountInterfaceIdempotentInstruction(
            payer,
            destination,
            recipient,
            mint,
            programId,
        );

    const finalBatch = batches[batches.length - 1];
    let insertionIdx = 0;
    while (
        insertionIdx < finalBatch.length &&
        finalBatch[insertionIdx].programId.equals(
            ComputeBudgetProgram.programId,
        )
    ) {
        insertionIdx += 1;
    }

    const patchedFinalBatch = [
        ...finalBatch.slice(0, insertionIdx),
        ensureRecipientAtaIx,
        ...finalBatch.slice(insertionIdx),
    ];
    const numSigners = payer.equals(sender) ? 1 : 2;
    assertTransactionSizeWithinLimit(
        patchedFinalBatch,
        numSigners,
        'Final transfer batch',
    );
    return [...batches.slice(0, -1), patchedFinalBatch];
}

export {
    createTransferToAccountInterfaceInstructions,
    calculateTransferCU,
} from '../instructions/transfer-interface';
