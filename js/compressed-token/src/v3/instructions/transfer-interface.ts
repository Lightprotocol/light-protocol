import {
    ComputeBudgetProgram,
    PublicKey,
    TransactionInstruction,
    SystemProgram,
} from '@solana/web3.js';
import {
    Rpc,
    assertV2Enabled,
    LIGHT_TOKEN_PROGRAM_ID,
} from '@lightprotocol/stateless.js';
import {
    TOKEN_PROGRAM_ID,
    TOKEN_2022_PROGRAM_ID,
    createTransferCheckedInstruction,
    TokenAccountNotFoundError,
} from '@solana/spl-token';
import BN from 'bn.js';
import { getAssociatedTokenAddressInterface } from '../get-associated-token-address-interface';
import {
    _buildLoadBatches,
    calculateLoadBatchComputeUnits,
    type InternalLoadBatch,
} from './load-ata';
import {
    getAtaInterface as _getAtaInterface,
    checkNotFrozen,
    type AccountInterface,
    spendableAmountForAuthority,
    isAuthorityForInterface,
    filterInterfaceForAuthority,
} from '../get-account-interface';
import { assertTransactionSizeWithinLimit } from '../utils/estimate-tx-size';
import type { InterfaceOptions } from '../actions/transfer-interface';
import { calculateCombinedCU } from './calculate-combined-cu';

const LIGHT_TOKEN_TRANSFER_DISCRIMINATOR = 3;
const LIGHT_TOKEN_TRANSFER_CHECKED_DISCRIMINATOR = 12;

const TRANSFER_BASE_CU = 10_000;
const TRANSFER_EXTRA_BUFFER_CU = 10_000;

export function calculateTransferCU(
    loadBatch: InternalLoadBatch | null,
): number {
    return calculateCombinedCU(
        TRANSFER_BASE_CU + TRANSFER_EXTRA_BUFFER_CU,
        loadBatch,
    );
}

/**
 * Create a light-token transfer instruction.
 *
 * For light-token accounts with compressible extension, the program needs
 * system_program and fee_payer to handle rent top-ups.
 *
 * @param source        Source light-token account
 * @param destination   Destination light-token account
 * @param owner         Owner of the source account (signer, also pays for compressible extension top-ups)
 * @param amount        Amount to transfer
 * @param feePayer      Optional fee payer for top-ups (defaults to owner)
 * @returns Transaction instruction for light-token transfer
 */
export function createLightTokenTransferInstruction(
    source: PublicKey,
    destination: PublicKey,
    owner: PublicKey,
    amount: number | bigint,
    feePayer?: PublicKey,
): TransactionInstruction {
    // Instruction data format:
    // byte 0: discriminator (3)
    // bytes 1-8: amount (u64 LE)
    const data = Buffer.alloc(9);
    data.writeUInt8(LIGHT_TOKEN_TRANSFER_DISCRIMINATOR, 0);
    data.writeBigUInt64LE(BigInt(amount), 1);

    const effectiveFeePayer = feePayer ?? owner;

    // Account order per program:
    // 0: source (writable)
    // 1: destination (writable)
    // 2: authority/owner (signer, writable only when paying top-ups)
    // 3: system_program (for top-ups via CPI)
    // 4: fee_payer (signer, writable - pays for top-ups)
    const keys = [
        { pubkey: source, isSigner: false, isWritable: true },
        { pubkey: destination, isSigner: false, isWritable: true },
        {
            pubkey: owner,
            isSigner: true,
            isWritable: effectiveFeePayer.equals(owner),
        },
        { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
        {
            pubkey: effectiveFeePayer,
            isSigner: !effectiveFeePayer.equals(owner), // Only mark as signer if different from owner (owner already signed)
            isWritable: true,
        },
    ];

    return new TransactionInstruction({
        programId: LIGHT_TOKEN_PROGRAM_ID,
        keys,
        data,
    });
}

/**
 * Create a light-token transfer_checked instruction. Same semantics as SPL
 * TransferChecked.
 */
export function createLightTokenTransferCheckedInstruction(
    source: PublicKey,
    destination: PublicKey,
    mint: PublicKey,
    owner: PublicKey,
    amount: number | bigint,
    decimals: number,
    payer: PublicKey,
): TransactionInstruction {
    const data = Buffer.alloc(10);
    data.writeUInt8(LIGHT_TOKEN_TRANSFER_CHECKED_DISCRIMINATOR, 0);
    data.writeBigUInt64LE(BigInt(amount), 1);
    data.writeUInt8(decimals, 9);

    const keys = [
        { pubkey: source, isSigner: false, isWritable: true },
        { pubkey: mint, isSigner: false, isWritable: false },
        { pubkey: destination, isSigner: false, isWritable: true },
        { pubkey: owner, isSigner: true, isWritable: payer.equals(owner) },
        { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
        {
            pubkey: payer,
            isSigner: !payer.equals(owner),
            isWritable: true,
        },
    ];

    return new TransactionInstruction({
        programId: LIGHT_TOKEN_PROGRAM_ID,
        keys,
        data,
    });
}

export async function createTransferToAccountInterfaceInstructions(
    rpc: Rpc,
    payer: PublicKey,
    mint: PublicKey,
    amount: number | bigint | BN,
    owner: PublicKey,
    destination: PublicKey,
    decimals: number,
    options?: InterfaceOptions,
    programId: PublicKey = LIGHT_TOKEN_PROGRAM_ID,
): Promise<TransactionInstruction[][]> {
    assertV2Enabled();

    const amountBigInt = BigInt(amount.toString());

    if (amountBigInt <= BigInt(0)) {
        throw new Error('Transfer amount must be greater than zero.');
    }

    const wrap = options?.wrap ?? false;
    const delegatePubkey = options?.delegatePubkey;
    const effectiveOwner = owner;
    const authorityPubkey = delegatePubkey ?? effectiveOwner;

    const isSplOrT22 =
        programId.equals(TOKEN_PROGRAM_ID) ||
        programId.equals(TOKEN_2022_PROGRAM_ID);

    const senderAta = getAssociatedTokenAddressInterface(
        mint,
        effectiveOwner,
        false,
        programId,
    );

    let senderInterface: AccountInterface;
    try {
        senderInterface = await _getAtaInterface(
            rpc,
            senderAta,
            effectiveOwner,
            mint,
            undefined,
            programId.equals(LIGHT_TOKEN_PROGRAM_ID) ? undefined : programId,
            wrap,
        );
    } catch (error) {
        if (error instanceof TokenAccountNotFoundError) {
            throw new Error('Sender has no token accounts for this mint.');
        }
        throw error;
    }

    checkNotFrozen(senderInterface, 'transfer');

    const isDelegate = !effectiveOwner.equals(authorityPubkey);
    if (isDelegate) {
        if (!isAuthorityForInterface(senderInterface, authorityPubkey)) {
            throw new Error(
                'Signer is not the owner or a delegate of the sender account.',
            );
        }
        const spendable = spendableAmountForAuthority(
            senderInterface,
            authorityPubkey,
        );
        if (amountBigInt > spendable) {
            throw new Error(
                `Insufficient delegated balance. Required: ${amountBigInt}, Available (delegate): ${spendable}`,
            );
        }
        senderInterface = filterInterfaceForAuthority(
            senderInterface,
            authorityPubkey,
        );
    } else {
        if (senderInterface.parsed.amount < amountBigInt) {
            throw new Error(
                `Insufficient balance. Required: ${amountBigInt}, Available: ${senderInterface.parsed.amount}`,
            );
        }
    }

    const internalBatches = await _buildLoadBatches(
        rpc,
        payer,
        senderInterface,
        options,
        wrap,
        senderAta,
        amountBigInt,
        authorityPubkey,
        decimals,
    );

    let transferIx: TransactionInstruction;
    if (isSplOrT22 && !wrap) {
        transferIx = createTransferCheckedInstruction(
            senderAta,
            mint,
            destination,
            authorityPubkey,
            amountBigInt,
            decimals,
            [],
            programId,
        );
    } else {
        transferIx = createLightTokenTransferCheckedInstruction(
            senderAta,
            destination,
            mint,
            authorityPubkey,
            amountBigInt,
            decimals,
            payer,
        );
    }

    const numSigners = payer.equals(authorityPubkey) ? 1 : 2;

    if (internalBatches.length === 0) {
        const cu = calculateTransferCU(null);
        const txIxs = [
            ComputeBudgetProgram.setComputeUnitLimit({ units: cu }),
            transferIx,
        ];
        assertTransactionSizeWithinLimit(txIxs, numSigners, 'Batch');
        return [txIxs];
    }

    if (internalBatches.length === 1) {
        const batch = internalBatches[0];
        const cu = calculateTransferCU(batch);
        const txIxs = [
            ComputeBudgetProgram.setComputeUnitLimit({ units: cu }),
            ...batch.instructions,
            transferIx,
        ];
        assertTransactionSizeWithinLimit(txIxs, numSigners, 'Batch');
        return [txIxs];
    }

    const result: TransactionInstruction[][] = [];

    for (let i = 0; i < internalBatches.length - 1; i++) {
        const batch = internalBatches[i];
        const cu = calculateLoadBatchComputeUnits(batch);
        const txIxs = [
            ComputeBudgetProgram.setComputeUnitLimit({ units: cu }),
            ...batch.instructions,
        ];
        assertTransactionSizeWithinLimit(txIxs, numSigners, 'Batch');
        result.push(txIxs);
    }

    const lastBatch = internalBatches[internalBatches.length - 1];
    const lastCu = calculateTransferCU(lastBatch);
    const lastTxIxs = [
        ComputeBudgetProgram.setComputeUnitLimit({ units: lastCu }),
        ...lastBatch.instructions,
        transferIx,
    ];
    assertTransactionSizeWithinLimit(lastTxIxs, numSigners, 'Batch');
    result.push(lastTxIxs);

    return result;
}
