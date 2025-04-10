import {
    ParsedMessageAccount,
    ParsedTransactionWithMeta,
    PublicKey,
} from '@solana/web3.js';
import bs58 from 'bs58';
import {
    COMPUTE_BUDGET_PATTERN,
    defaultStaticAccountsStruct,
    INSERT_INTO_QUEUES_DISCRIMINATOR,
    INVOKE_CPI_DISCRIMINATOR,
    INVOKE_CPI_WITH_READ_ONLY_DISCRIMINATOR,
    INVOKE_DISCRIMINATOR,
} from '../../constants';
import {
    convertToPublicTransactionEvent,
    decodeInstructionDataInvoke,
    decodeInstructionDataInvokeCpi,
    deserializeAppendNullifyCreateAddressInputsIndexer,
} from '../../programs';
import { Rpc } from '../../rpc';
import { InstructionDataInvoke, PublicTransactionEvent } from '../../state';
import {
    decodeInstructionDataInvokeCpiWithReadOnly,
    decodePublicTransactionEvent,
} from '../../programs/system/layout';
import { Buffer } from 'buffer';
import { convertInvokeCpiWithReadOnlyToInvoke } from '../../utils';

type Deserializer<T> = (data: Buffer, tx: ParsedTransactionWithMeta) => T;

/**
 * @internal
 * Returns newest first.
 *
 * */
export async function getParsedEvents(
    rpc: Rpc,
): Promise<PublicTransactionEvent[]> {
    const events: PublicTransactionEvent[] = [];

    const { noopProgram, accountCompressionProgram } =
        defaultStaticAccountsStruct();

    const signatures = (
        await rpc.getSignaturesForAddress(
            accountCompressionProgram,
            undefined,
            'confirmed',
        )
    ).map(s => s.signature);
    const txs = await rpc.getParsedTransactions(signatures, {
        maxSupportedTransactionVersion: 0,
        commitment: 'confirmed',
    });

    for (const txParsed of txs) {
        if (!txParsed || !txParsed.transaction || !txParsed.meta) continue;

        if (
            !txParsed.meta.innerInstructions ||
            txParsed.meta.innerInstructions.length == 0
        ) {
            continue;
        }

        const messageV0 = txParsed.transaction.message;
        const accKeys = messageV0.accountKeys;

        const allAccounts = accKeys.map(a => a.pubkey);
        const dataVec: Uint8Array[] = [];

        // get tx wth sig
        const txRaw = await rpc.getTransaction(
            txParsed.transaction.signatures[0],
            {
                commitment: 'confirmed',
                maxSupportedTransactionVersion: 0,
            },
        );

        for (const ix of txRaw?.transaction.message.compiledInstructions ||
            []) {
            if (ix.data && ix.data.length > 0) {
                const decodedData = Uint8Array.from(ix.data);
                if (
                    decodedData.length === COMPUTE_BUDGET_PATTERN.length &&
                    COMPUTE_BUDGET_PATTERN.every(
                        (byte, idx) => byte === decodedData[idx],
                    )
                ) {
                    continue;
                }
                dataVec.push(decodedData);
            }
        }

        const groupedAccountVec: PublicKey[][] = [];

        if (
            txRaw!.meta!.innerInstructions &&
            txRaw!.meta!.innerInstructions.length > 0
        ) {
            for (const innerGroup of txRaw!.meta!.innerInstructions) {
                for (const ix of innerGroup.instructions) {
                    const group = ix.accounts.map(
                        (accountIdx: number) => allAccounts[accountIdx],
                    );
                    groupedAccountVec.push(group);
                    if (ix.data && ix.data.length > 0) {
                        const decodedData = bs58.decode(ix.data);
                        dataVec.push(decodedData);
                    }
                }
            }
        }

        const event = parseLightTransaction(dataVec, groupedAccountVec);
        if (event) {
            events.push(event);
        }
    }

    if (events.length > 0) {
        return events;
    }

    /// Filter by NOOP program
    const transactionEvents = txs.filter(
        (tx: ParsedTransactionWithMeta | null) => {
            if (!tx) {
                return false;
            }
            const accountKeys = tx.transaction.message.accountKeys;

            const hasSplNoopAddress = accountKeys.some(
                (item: ParsedMessageAccount) => {
                    const itemStr =
                        typeof item === 'string'
                            ? item
                            : item.pubkey.toBase58();
                    return itemStr === noopProgram.toBase58();
                },
            );

            return hasSplNoopAddress;
        },
    );

    return parseEvents(transactionEvents, parsePublicTransactionEventWithIdl);
}

export const parseEvents = <T>(
    indexerEventsTransactions: (ParsedTransactionWithMeta | null)[],
    deserializeFn: Deserializer<T>,
): NonNullable<T>[] => {
    const { noopProgram } = defaultStaticAccountsStruct();

    const transactions: NonNullable<T>[] = [];
    indexerEventsTransactions.forEach(tx => {
        if (
            !tx ||
            !tx.meta ||
            tx.meta.err ||
            !tx.meta.innerInstructions ||
            tx.meta.innerInstructions.length <= 0
        ) {
            return;
        }

        /// We only care about the very last inner instruction as it contains the
        /// PublicTransactionEvent
        tx.meta.innerInstructions.forEach(ix => {
            if (ix.instructions.length > 0) {
                const ixInner = ix.instructions[ix.instructions.length - 1];
                // Type guard for partially parsed web3js types.
                if (
                    'data' in ixInner &&
                    ixInner.data &&
                    ixInner.programId.toBase58() === noopProgram.toBase58()
                ) {
                    const data = bs58.decode(ixInner.data);

                    const decodedEvent = deserializeFn(Buffer.from(data), tx);

                    if (decodedEvent !== null && decodedEvent !== undefined) {
                        transactions.push(decodedEvent as NonNullable<T>);
                    }
                }
            }
        });
    });

    return transactions;
};

// TODO: make it type safe. have to reimplement the types from the IDL.
export const parsePublicTransactionEventWithIdl = (
    data: Buffer,
): PublicTransactionEvent | null => {
    const numericData = Buffer.from(data.map(byte => byte));

    try {
        return decodePublicTransactionEvent(numericData);
    } catch (error) {
        console.error('Error deserializing event:', error);
        return null;
    }
};

export function parseLightTransaction(
    dataVec: Uint8Array[],
    accountKeys: PublicKey[][],
): PublicTransactionEvent | null | undefined {
    let foundSystemInstruction = false;

    let invokeData: InstructionDataInvoke | null = null;
    let appendInputsData = null;

    // First pass for system instructions
    for (const data of dataVec) {
        const discriminator = data.slice(0, 8);
        const discriminatorStr = bs58.encode(discriminator);
        const invokeDiscriminatorStr = bs58.encode(INVOKE_DISCRIMINATOR);
        const invokeCpiDiscriminatorStr = bs58.encode(INVOKE_CPI_DISCRIMINATOR);
        const invokeCpiWithReadOnlyDiscriminatorStr = bs58.encode(
            INVOKE_CPI_WITH_READ_ONLY_DISCRIMINATOR,
        );
        if (discriminatorStr === invokeDiscriminatorStr) {
            invokeData = decodeInstructionDataInvoke(Buffer.from(data));
            foundSystemInstruction = true;
            break;
        }
        if (discriminatorStr == invokeCpiDiscriminatorStr) {
            invokeData = decodeInstructionDataInvokeCpi(Buffer.from(data));
            foundSystemInstruction = true;
            break;
        }
        if (discriminatorStr == invokeCpiWithReadOnlyDiscriminatorStr) {
            const decoded = decodeInstructionDataInvokeCpiWithReadOnly(
                Buffer.from(data),
            );
            invokeData = convertInvokeCpiWithReadOnlyToInvoke(decoded);
            foundSystemInstruction = true;
            break;
        }
    }
    if (!foundSystemInstruction) return null;

    for (const data of dataVec) {
        const discriminator = data.slice(0, 8);
        const discriminatorStr = bs58.encode(discriminator);
        const insertIntoQueuesDiscriminatorStr = bs58.encode(
            INSERT_INTO_QUEUES_DISCRIMINATOR,
        );
        if (discriminatorStr === insertIntoQueuesDiscriminatorStr) {
            const dataSlice = data.slice(12);
            appendInputsData =
                deserializeAppendNullifyCreateAddressInputsIndexer(
                    Buffer.from(dataSlice),
                );
        }
    }

    if (invokeData) {
        return convertToPublicTransactionEvent(
            appendInputsData,
            accountKeys[accountKeys.length - 1],
            invokeData,
        );
    } else {
        return null;
    }
}
