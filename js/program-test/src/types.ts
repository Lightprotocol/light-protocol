/**
 * Configuration options for LiteSVM test environment
 */
export interface LiteSVMConfig {
    /** Enable signature verification */
    sigverify?: boolean;
    /** Enable blockhash checking */
    blockhashCheck?: boolean;
    /** Initial lamports for the test environment */
    initialLamports?: bigint;
    /** Transaction history size */
    transactionHistorySize?: bigint;
}
