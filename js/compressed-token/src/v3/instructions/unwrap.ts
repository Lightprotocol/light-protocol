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
import { TokenAccountNotFoundError } from '@solana/spl-token';
import type BN from 'bn.js';
import { MAX_TOP_UP } from '../../constants';
import { CompressedTokenProgram } from '../../program';
import {
    getSplInterfaceInfos,
    SplInterfaceInfo,
} from '../../utils/get-token-pool-infos';
import { getAssociatedTokenAddressInterface } from '../get-associated-token-address-interface';
import {
    getAtaInterface as _getAtaInterface,
    checkNotFrozen,
    type AccountInterface,
} from '../get-account-interface';
import {
    _buildLoadBatches,
    calculateLoadBatchComputeUnits,
    type InternalLoadBatch,
} from './load-ata';
import type { InterfaceOptions } from '../actions/transfer-interface';
import { assertTransactionSizeWithinLimit } from '../utils/estimate-tx-size';
import { ERR_NO_LIGHT_TOKEN_BALANCE_UNWRAP } from '../errors';
import {
    encodeTransfer2InstructionData,
    createCompressLightToken,
    createDecompressSpl,
    Transfer2InstructionData,
    Compression,
} from '../layout/layout-transfer2';
import { calculateCombinedCU } from './calculate-combined-cu';

const UNWRAP_BASE_CU = 10_000;

function calculateUnwrapCU(loadBatch: InternalLoadBatch | null): number {
    return calculateCombinedCU(UNWRAP_BASE_CU, loadBatch);
}

/**
 * Create an unwrap instruction that moves tokens from a light-token account to an
 * SPL/T22 account.
 *
 * @param source           Source light-token account
 * @param destination      Destination SPL/T22 token account
 * @param owner            Owner of the source account (signer)
 * @param mint             Mint address
 * @param amount           Amount to unwrap,
 * @param splInterfaceInfo SPL interface info for the decompression
 * @param decimals         Mint decimals (required for transfer_checked)
 * @param payer            Fee payer (defaults to owner if not provided)
 * @param maxTopUp         Optional cap on rent top-up (units of 1k lamports; default no cap)
 * @returns TransactionInstruction to unwrap tokens
 */
export function createUnwrapInstruction(
    source: PublicKey,
    destination: PublicKey,
    owner: PublicKey,
    mint: PublicKey,
    amount: bigint,
    splInterfaceInfo: SplInterfaceInfo,
    decimals: number,
    payer: PublicKey = owner,
    maxTopUp?: number,
): TransactionInstruction {
    const MINT_INDEX = 0;
    const OWNER_INDEX = 1;
    const SOURCE_INDEX = 2;
    const DESTINATION_INDEX = 3;
    const POOL_INDEX = 4;
    const _SPL_TOKEN_PROGRAM_INDEX = 5;
    const LIGHT_TOKEN_PROGRAM_INDEX = 6;

    // Unwrap flow: compress from light-token, decompress to SPL
    const compressions: Compression[] = [
        createCompressLightToken(
            amount,
            MINT_INDEX,
            SOURCE_INDEX,
            OWNER_INDEX,
            LIGHT_TOKEN_PROGRAM_INDEX,
        ),
        createDecompressSpl(
            amount,
            MINT_INDEX,
            DESTINATION_INDEX,
            POOL_INDEX,
            splInterfaceInfo.poolIndex,
            splInterfaceInfo.bump,
            decimals,
        ),
    ];

    const instructionData: Transfer2InstructionData = {
        withTransactionHash: false,
        withLamportsChangeAccountMerkleTreeIndex: false,
        lamportsChangeAccountMerkleTreeIndex: 0,
        lamportsChangeAccountOwnerIndex: 0,
        outputQueue: 0,
        maxTopUp: maxTopUp ?? MAX_TOP_UP,
        cpiContext: null,
        compressions,
        proof: null,
        inTokenData: [],
        outTokenData: [],
        inLamports: null,
        outLamports: null,
        inTlv: null,
        outTlv: null,
    };

    const data = encodeTransfer2InstructionData(instructionData);

    // Account order matches wrap instruction for consistency
    const keys = [
        {
            pubkey: CompressedTokenProgram.deriveCpiAuthorityPda,
            isSigner: false,
            isWritable: false,
        },
        { pubkey: payer, isSigner: true, isWritable: true },
        { pubkey: mint, isSigner: false, isWritable: false },
        { pubkey: owner, isSigner: true, isWritable: false },
        { pubkey: source, isSigner: false, isWritable: true },
        { pubkey: destination, isSigner: false, isWritable: true },
        {
            pubkey: splInterfaceInfo.splInterfacePda,
            isSigner: false,
            isWritable: true,
        },
        {
            pubkey: splInterfaceInfo.tokenProgram,
            isSigner: false,
            isWritable: false,
        },
        {
            pubkey: LIGHT_TOKEN_PROGRAM_ID,
            isSigner: false,
            isWritable: false,
        },
        // System program needed for top-up CPIs when source has compressible extension
        {
            pubkey: SystemProgram.programId,
            isSigner: false,
            isWritable: false,
        },
    ];

    return new TransactionInstruction({
        programId: CompressedTokenProgram.programId,
        keys,
        data,
    });
}

export async function createUnwrapInstructions(
    rpc: Rpc,
    destination: PublicKey,
    owner: PublicKey,
    mint: PublicKey,
    decimals: number,
    amount?: number | bigint | BN,
    payer?: PublicKey,
    splInterfaceInfo?: SplInterfaceInfo,
    maxTopUp?: number,
    interfaceOptions?: InterfaceOptions,
    wrap = false,
): Promise<TransactionInstruction[][]> {
    assertV2Enabled();

    payer ??= owner;

    let resolvedSplInterfaceInfo = splInterfaceInfo;
    if (!resolvedSplInterfaceInfo) {
        const splInterfaceInfos = await getSplInterfaceInfos(rpc, mint);
        resolvedSplInterfaceInfo = splInterfaceInfos.find(
            info => info.isInitialized,
        );

        if (!resolvedSplInterfaceInfo) {
            throw new Error(
                `No initialized SPL interface found for mint: ${mint.toBase58()}. ` +
                    `Please create an SPL interface via createSplInterface().`,
            );
        }
    }

    const destAtaInfo = await rpc.getAccountInfo(destination);
    if (!destAtaInfo) {
        throw new Error(
            `Destination account does not exist: ${destination.toBase58()}. ` +
                `Create it first using getOrCreateAssociatedTokenAccount or createAssociatedTokenAccountIdempotentInstruction.`,
        );
    }

    const lightTokenAta = getAssociatedTokenAddressInterface(mint, owner);

    let accountInterface: AccountInterface;
    try {
        accountInterface = await _getAtaInterface(
            rpc,
            lightTokenAta,
            owner,
            mint,
            undefined,
            undefined,
            wrap,
        );
    } catch (error) {
        if (error instanceof TokenAccountNotFoundError) {
            throw new Error(ERR_NO_LIGHT_TOKEN_BALANCE_UNWRAP);
        }
        throw error;
    }

    checkNotFrozen(accountInterface, 'unwrap');

    const totalBalance = accountInterface.parsed.amount;
    if (totalBalance === BigInt(0)) {
        throw new Error(ERR_NO_LIGHT_TOKEN_BALANCE_UNWRAP);
    }

    const unwrapAmount =
        amount != null ? BigInt(amount.toString()) : totalBalance;

    if (unwrapAmount === BigInt(0)) {
        throw new Error('Unwrap amount must be greater than zero.');
    }

    if (unwrapAmount > totalBalance) {
        throw new Error(
            `Insufficient light-token balance. Requested: ${unwrapAmount}, Available: ${totalBalance}`,
        );
    }

    const internalBatches = await _buildLoadBatches(
        rpc,
        payer,
        accountInterface,
        interfaceOptions,
        wrap,
        lightTokenAta,
        amount != null ? unwrapAmount : undefined,
        undefined,
        decimals,
    );

    const ix = createUnwrapInstruction(
        lightTokenAta,
        destination,
        owner,
        mint,
        unwrapAmount,
        resolvedSplInterfaceInfo,
        decimals,
        payer,
        maxTopUp,
    );

    const numSigners = payer.equals(owner) ? 1 : 2;
    if (internalBatches.length === 0) {
        const txIxs = [
            ComputeBudgetProgram.setComputeUnitLimit({
                units: calculateUnwrapCU(null),
            }),
            ix,
        ];
        assertTransactionSizeWithinLimit(txIxs, numSigners, 'Unwrap batch');
        return [txIxs];
    }

    if (internalBatches.length === 1) {
        const batch = internalBatches[0];
        const txIxs = [
            ComputeBudgetProgram.setComputeUnitLimit({
                units: calculateUnwrapCU(batch),
            }),
            ...batch.instructions,
            ix,
        ];
        assertTransactionSizeWithinLimit(txIxs, numSigners, 'Unwrap batch');
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
        assertTransactionSizeWithinLimit(txIxs, numSigners, 'Unwrap batch');
        result.push(txIxs);
    }

    const lastBatch = internalBatches[internalBatches.length - 1];
    const lastTxIxs = [
        ComputeBudgetProgram.setComputeUnitLimit({
            units: calculateUnwrapCU(lastBatch),
        }),
        ...lastBatch.instructions,
        ix,
    ];
    assertTransactionSizeWithinLimit(lastTxIxs, numSigners, 'Unwrap batch');
    result.push(lastTxIxs);

    return result;
}
