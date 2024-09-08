import {
    ParsedMessageAccount,
    ParsedTransactionWithMeta,
} from '@solana/web3.js';
import { bs58 } from '@coral-xyz/anchor/dist/cjs/utils/bytes';
import { defaultStaticAccountsStruct } from '../../constants';
import { LightSystemProgram } from '../../programs';
import { Rpc } from '../../rpc';
import { PublicTransactionEvent } from '../../state';

type Deserializer<T> = (data: Buffer, tx: ParsedTransactionWithMeta) => T;

/**
 * @internal
 * Returns newest first.
 *
 * */
export async function getParsedEvents(
    rpc: Rpc,
): Promise<PublicTransactionEvent[]> {
    const { noopProgram, accountCompressionProgram } =
        defaultStaticAccountsStruct();

    /// Get raw transactions
    const signatures = (
        await rpc.getConfirmedSignaturesForAddress2(
            accountCompressionProgram,
            undefined,
            'confirmed',
        )
    ).map(s => s.signature);
    const txs = await rpc.getParsedTransactions(signatures, {
        maxSupportedTransactionVersion: 0,
        commitment: 'confirmed',
    });

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

    /// Parse events
    const parsedEvents = parseEvents(
        transactionEvents,
        parsePublicTransactionEventWithIdl,
    );

    return parsedEvents;
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
        return LightSystemProgram.program.coder.types.decode(
            'publicTransactionEvent',
            numericData,
        );
    } catch (error) {
        console.error('Error deserializing event:', error);
        return null;
    }
};
