import { bs58 } from '@coral-xyz/anchor/dist/cjs/utils/bytes';
import { ParsedTransactionWithMeta } from '@solana/web3.js';
import { LightSystemProgram } from '../programs';
import { defaultStaticAccountsStruct } from '../constants';
import { PublicTransactionEvent_IdlType } from '../state';

type Deserializer<T> = (data: Buffer, tx: ParsedTransactionWithMeta) => T;

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

                    const decodedEvent = deserializeFn(data, tx);

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
): PublicTransactionEvent_IdlType | null => {
    const numericData = Buffer.from(data.map(byte => byte));

    try {
        return LightSystemProgram.program.coder.types.decode(
            'PublicTransactionEvent',
            numericData,
        );
    } catch (error) {
        console.error('Error deserializing event:', error);
        return null;
    }
};
