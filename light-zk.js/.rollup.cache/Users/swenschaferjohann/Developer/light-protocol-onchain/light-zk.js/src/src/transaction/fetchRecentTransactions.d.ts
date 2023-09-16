/// <reference types="node" />
import { ConfirmedSignaturesForAddress2Options, Connection, PublicKey } from "@solana/web3.js";
import { IndexedTransaction, UserIndexedTransaction, ParsedIndexedTransaction } from "../types";
import { TokenUtxoBalance, Provider } from "../wallet";
export declare class TransactionIndexerEvent {
    borshSchema: any;
    deserialize(buffer: Buffer): any | null;
}
/**
 *  Call Flow:
 *  fetchRecentTransactions() <-- called in indexer
 *    getTransactionsBatch()
 *      getSigsForAdd()
 *		    getTxForSig()
 *		      make Events:
 *			    parseTransactionEvents()
 *			    enrichParsedTransactionEvents()
 */
/**
 * @async
 * @description This functions takes the IndexedTransaction and spentUtxos of user any return the filtered user indexed transactions
 * @function getUserIndexTransactions
 * @param {IndexedTransaction[]} indexedTransactions - An array to which the processed transaction data will be pushed.
 * @param {Provider} provider - provider class
 * @param {spentUtxos} Utxo[] - The array of user spentUtxos
 * @returns {Promise<void>}
 */
export declare const getUserIndexTransactions: (indexedTransactions: ParsedIndexedTransaction[], provider: Provider, tokenBalances: Map<string, TokenUtxoBalance>) => Promise<UserIndexedTransaction[]>;
type Instruction = {
    accounts: any[];
    data: string;
    programId: PublicKey;
    stackHeight: null | number;
};
export declare const findMatchingInstruction: (instructions: Instruction[], publicKeys: PublicKey[]) => Instruction | undefined;
/**
 * @description Fetches recent transactions for the specified merkleTreeProgramId.
 * This function will call getTransactionsBatch multiple times to fetch transactions in batches.
 * @param {Connection} connection - The Connection object to interact with the Solana network.
 * @param {ConfirmedSignaturesForAddress2Options} batchOptions - Options for fetching transaction batches,
 * including starting transaction signature (after), ending transaction signature (before), and batch size (limit).
 * @param {boolean} dedupe=false - Whether to deduplicate transactions or not.
 * @returns {Promise<indexedTransaction[]>} Array of indexedTransactions
 */
export declare function fetchRecentTransactions({ connection, batchOptions, transactions, }: {
    connection: Connection;
    batchOptions: ConfirmedSignaturesForAddress2Options;
    transactions?: IndexedTransaction[];
}): Promise<IndexedTransaction[]>;
export {};
//# sourceMappingURL=fetchRecentTransactions.d.ts.map