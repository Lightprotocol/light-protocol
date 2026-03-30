export const ERR_FETCH_BY_OWNER_REQUIRED = 'fetchByOwner is required';

export class MultiTransactionNotSupportedError extends Error {
    readonly operation: string;
    readonly batchCount: number;

    constructor(operation: string, batchCount: number) {
        super(
            `${operation} requires ${batchCount} transactions with the current underlying interface builders. ` +
                '@lightprotocol/token-interface only exposes single-transaction instruction builders.',
        );
        this.name = 'MultiTransactionNotSupportedError';
        this.operation = operation;
        this.batchCount = batchCount;
    }
}
