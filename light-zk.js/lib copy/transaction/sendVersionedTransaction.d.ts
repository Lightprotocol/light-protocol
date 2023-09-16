/// <reference types="@coral-xyz/anchor/node_modules/@solana/web3.js" />
import { Commitment, Connection, PublicKey, TransactionInstruction, TransactionSignature } from "@solana/web3.js";
import { Wallet } from "../wallet";
export declare const sendVersionedTransaction: (ix: TransactionInstruction, connection: Connection, lookUpTable: PublicKey, payer: Wallet) => Promise<string | undefined>;
export type SendVersionedTransactionsResult = {
    signatures?: TransactionSignature[];
    error?: any;
};
export declare function sendVersionedTransactions(instructions: any[], connection: Connection, lookUpTable: PublicKey, payer: Wallet): Promise<SendVersionedTransactionsResult>;
export declare function confirmTransaction(connection: Connection, signature: string, confirmation?: Commitment): Promise<import("@solana/web3.js").RpcResponseAndContext<import("@solana/web3.js").SignatureResult>>;
