/// <reference types="node" />
/// <reference types="bn.js" />
import { PublicKey } from "@solana/web3.js";
import * as anchor from "@coral-xyz/anchor";
import { BN } from "@coral-xyz/anchor";
import { Provider, Account, Utxo, Transaction, TransactionParameters, Action, AppUtxoConfig, Balance, InboxBalance, TokenUtxoBalance, UserIndexedTransaction, ProgramUtxoBalance } from "../index";
import { Idl } from "@coral-xyz/anchor";
export declare enum ConfirmOptions {
    finalized = "finalized",
    spendable = "spendable"
}
export type UtxoBatch = {
    leftLeafIndex: number;
    encryptedUtxos: {
        index: number;
        commitment: Buffer;
        leftLeaf: Uint8Array;
        encBytes: Buffer | any[];
    }[];
};
/**
 *
 * @param provider Either a nodeProvider or browserProvider
 * @param account User account (optional)
 * @param utxos User utxos (optional)
 *
 */
export declare class User {
    provider: Provider;
    account: Account;
    transactionHistory?: UserIndexedTransaction[];
    recentTransactionParameters?: TransactionParameters;
    recentTransaction?: Transaction;
    approved?: boolean;
    appUtxoConfig?: AppUtxoConfig;
    balance: Balance;
    inboxBalance: InboxBalance;
    verifierIdl: Idl;
    constructor({ provider, account, appUtxoConfig, verifierIdl, }: {
        provider: Provider;
        serializedUtxos?: Buffer;
        serialiezdSpentUtxos?: Buffer;
        account: Account;
        appUtxoConfig?: AppUtxoConfig;
        verifierIdl?: Idl;
    });
    syncState(aes: boolean | undefined, balance: Balance | InboxBalance, merkleTreePdaPublicKey: PublicKey): Promise<Balance | InboxBalance>;
    /**
     * returns all non-accepted utxos.
     * would not be part of the main balance
     */
    getUtxoInbox(latest?: boolean): Promise<InboxBalance>;
    getBalance(latest?: boolean): Promise<Balance>;
    /**
     *
     * @param amount e.g. 1 SOL = 1, 2 USDC = 2
     * @param token "SOL", "USDC", "USDT",
     * @param recipient optional, if not set, will shield to self
     * @param extraSolAmount optional, if set, will add extra SOL to the shielded amount
     * @param senderTokenAccount optional, if set, will use this token account to shield from, else derives ATA
     */
    createShieldTransactionParameters({ token, publicAmountSpl, recipient, publicAmountSol, senderTokenAccount, minimumLamports, appUtxo, mergeExistingUtxos, verifierIdl, message, skipDecimalConversions, utxo, }: {
        token: string;
        recipient?: Account;
        publicAmountSpl?: number | BN | string;
        publicAmountSol?: number | BN | string;
        minimumLamports?: boolean;
        senderTokenAccount?: PublicKey;
        appUtxo?: AppUtxoConfig;
        mergeExistingUtxos?: boolean;
        verifierIdl?: Idl;
        message?: Buffer;
        skipDecimalConversions?: boolean;
        utxo?: Utxo;
    }): Promise<TransactionParameters>;
    compileAndProveTransaction(appParams?: any, shuffleEnabled?: boolean): Promise<Transaction>;
    approve(): Promise<void>;
    sendTransaction(): Promise<import("../index").SendVersionedTransactionsResult | import("../relayer").RelayerSendTransactionsResponse>;
    resetTxState(): void;
    /**
     *
     * @param amount e.g. 1 SOL = 1, 2 USDC = 2
     * @param token "SOL", "USDC", "USDT",
     * @param recipient optional, if not set, will shield to self
     * @param extraSolAmount optional, if set, will add extra SOL to the shielded amount
     * @param senderTokenAccount optional, if set, will use this token account to shield from, else derives ATA
     */
    shield({ token, publicAmountSpl, recipient, publicAmountSol, senderTokenAccount, minimumLamports, appUtxo, skipDecimalConversions, confirmOptions, }: {
        token: string;
        recipient?: string;
        publicAmountSpl?: number | BN | string;
        publicAmountSol?: number | BN | string;
        minimumLamports?: boolean;
        senderTokenAccount?: PublicKey;
        appUtxo?: AppUtxoConfig;
        skipDecimalConversions?: boolean;
        confirmOptions?: ConfirmOptions;
    }): Promise<{
        txHash: import("../index").SendVersionedTransactionsResult | import("../relayer").RelayerSendTransactionsResponse;
        response: string;
    }>;
    unshield({ token, publicAmountSpl, publicAmountSol, recipient, minimumLamports, confirmOptions, }: {
        token: string;
        recipient?: PublicKey;
        publicAmountSpl?: number | BN | string;
        publicAmountSol?: number | BN | string;
        minimumLamports?: boolean;
        confirmOptions?: ConfirmOptions;
    }): Promise<{
        txHash: import("../index").SendVersionedTransactionsResult | import("../relayer").RelayerSendTransactionsResponse;
        response: string;
    }>;
    /**
     * @params token: string
     * @params amount: number - in base units (e.g. lamports for 'SOL')
     * @params recipient: PublicKey - Solana address
     * @params extraSolAmount: number - optional, if not set, will use MINIMUM_LAMPORTS
     */
    createUnshieldTransactionParameters({ token, publicAmountSpl, publicAmountSol, recipient, minimumLamports, }: {
        token: string;
        recipient?: PublicKey;
        publicAmountSpl?: number | BN | string;
        publicAmountSol?: number | BN | string;
        minimumLamports?: boolean;
    }): Promise<TransactionParameters>;
    transfer({ token, recipient, amountSpl, amountSol, appUtxo, confirmOptions, }: {
        token: string;
        amountSpl?: BN | number | string;
        amountSol?: BN | number | string;
        recipient: string;
        appUtxo?: AppUtxoConfig;
        confirmOptions?: ConfirmOptions;
    }): Promise<{
        txHash: import("../index").SendVersionedTransactionsResult | import("../relayer").RelayerSendTransactionsResponse;
        response: string;
    }>;
    /**
     * @description transfers to one recipient utxo and creates a change utxo with remainders of the input
     * @param token mint
     * @param amount
     * @param recipient shieldedAddress (BN)
     * @param recipientEncryptionPublicKey (use strToArr)
     * @returns
     */
    createTransferTransactionParameters({ token, recipient, amountSpl, amountSol, appUtxo, message, inUtxos, outUtxos, verifierIdl, skipDecimalConversions, addInUtxos, addOutUtxos, }: {
        token?: string;
        amountSpl?: BN | number | string;
        amountSol?: BN | number | string;
        recipient?: Account;
        appUtxo?: AppUtxoConfig;
        message?: Buffer;
        inUtxos?: Utxo[];
        outUtxos?: Utxo[];
        verifierIdl?: Idl;
        skipDecimalConversions?: boolean;
        addInUtxos?: boolean;
        addOutUtxos?: boolean;
    }): Promise<TransactionParameters>;
    transactWithParameters({ txParams, appParams, confirmOptions, shuffleEnabled, }: {
        txParams: TransactionParameters;
        appParams?: any;
        confirmOptions?: ConfirmOptions;
        shuffleEnabled?: boolean;
    }): Promise<{
        txHash: import("../index").SendVersionedTransactionsResult | import("../relayer").RelayerSendTransactionsResponse;
        response: string;
    }>;
    transactWithUtxos({}: {
        inUtxos: Utxo[];
        outUtxos: Utxo[];
        action: Action;
        inUtxoCommitments: string[];
    }): Promise<void>;
    /**
     *
     * @param provider - Light provider
     * @param seed - Optional user seed to instantiate from; e.g. if the seed is supplied, skips the log-in signature prompt.
     * @param utxos - Optional user utxos to instantiate from
     */
    static init({ provider, seed, appUtxoConfig, account, skipFetchBalance, }: {
        provider: Provider;
        seed?: string;
        utxos?: Utxo[];
        appUtxoConfig?: AppUtxoConfig;
        account?: Account;
        skipFetchBalance?: boolean;
    }): Promise<any>;
    /** shielded transfer to self, merge 10-1;
     * get utxo inbox
     * merge highest first
     * loops in steps of 9 or 10
     */
    mergeAllUtxos(asset: PublicKey, confirmOptions?: ConfirmOptions, latest?: boolean): Promise<{
        txHash: import("../index").SendVersionedTransactionsResult | import("../relayer").RelayerSendTransactionsResponse;
        response: string;
    }>;
    /** shielded transfer to self, merge 10-1;
     * get utxo inbox
     * merge highest first
     * loops in steps of 9 or 10
     */
    mergeUtxos(commitments: string[], asset: PublicKey, confirmOptions?: ConfirmOptions, latest?: boolean): Promise<{
        txHash: import("../index").SendVersionedTransactionsResult | import("../relayer").RelayerSendTransactionsResponse;
        response: string;
    }>;
    getTransactionHistory(latest?: boolean): Promise<UserIndexedTransaction[]>;
    getUtxoStatus(): void;
    addUtxos(): void;
    createStoreAppUtxoTransactionParameters({ token, amountSol, amountSpl, minimumLamports, senderTokenAccount, recipientPublicKey, appUtxo, stringUtxo, action, appUtxoConfig, skipDecimalConversions, }: {
        token?: string;
        amountSol?: BN;
        amountSpl?: BN;
        minimumLamports?: boolean;
        senderTokenAccount?: PublicKey;
        recipientPublicKey?: string;
        appUtxo?: Utxo;
        stringUtxo?: string;
        action: Action;
        appUtxoConfig?: AppUtxoConfig;
        skipDecimalConversions?: boolean;
    }): Promise<TransactionParameters>;
    /**
     * is shield or transfer
     */
    storeAppUtxo({ token, amountSol, amountSpl, minimumLamports, senderTokenAccount, recipientPublicKey, appUtxo, stringUtxo, action, appUtxoConfig, skipDecimalConversions, confirmOptions, }: {
        token?: string;
        amountSol?: BN;
        amountSpl?: BN;
        minimumLamports?: boolean;
        senderTokenAccount?: PublicKey;
        recipientPublicKey?: string;
        appUtxo?: Utxo;
        stringUtxo?: string;
        action: Action;
        appUtxoConfig?: AppUtxoConfig;
        skipDecimalConversions?: boolean;
        confirmOptions?: ConfirmOptions;
    }): Promise<{
        txHash: import("../index").SendVersionedTransactionsResult | import("../relayer").RelayerSendTransactionsResponse;
        response: string;
    }>;
    /**
     * - get indexed transactions for a storage compressed account
     * - try to decrypt all and add to appUtxos or decrypted data map
     * - add custom descryption strategies for arbitrary data
     */
    syncStorage(idl: anchor.Idl, aes?: boolean): Promise<Map<string, ProgramUtxoBalance> | undefined>;
    getAllUtxos(): Utxo[];
    /**
     *
     */
    storeData(message: Buffer, confirmOptions?: ConfirmOptions, shield?: boolean): Promise<{
        txHash: import("../index").SendVersionedTransactionsResult | import("../relayer").RelayerSendTransactionsResponse;
        response: string;
    }>;
    executeAppUtxo({ appUtxos, inUtxos, outUtxos, action, programParameters, confirmOptions, addInUtxos, addOutUtxos, shuffleEnabled, }: {
        appUtxos?: Utxo[];
        outUtxos?: Utxo[];
        action: Action;
        programParameters: any;
        recipient?: Account;
        confirmOptions?: ConfirmOptions;
        addInUtxos?: boolean;
        addOutUtxos?: boolean;
        inUtxos?: Utxo[];
        shuffleEnabled?: boolean;
    }): Promise<{
        txHash: import("../index").SendVersionedTransactionsResult | import("../relayer").RelayerSendTransactionsResponse;
        response: string;
    }>;
    getProgramUtxos({ latestBalance, latestInboxBalance, idl, asMap, }: {
        latestBalance?: boolean;
        latestInboxBalance?: boolean;
        idl: Idl;
        aes?: boolean;
        asMap?: boolean;
    }): Promise<{
        tokenBalances: Map<string, TokenUtxoBalance> | undefined;
        inboxTokenBalances: Map<string, TokenUtxoBalance> | undefined;
        programUtxoArray?: undefined;
        inboxProgramUtxoArray?: undefined;
    } | {
        programUtxoArray: Utxo[];
        inboxProgramUtxoArray: Utxo[];
        tokenBalances?: undefined;
        inboxTokenBalances?: undefined;
    }>;
    getUtxo(commitment: string, latest?: boolean, idl?: Idl): Promise<{
        utxo: Utxo;
        status: string;
    } | undefined>;
}
