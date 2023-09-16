/// <reference types="bn.js" />
import { PublicKey } from "@solana/web3.js";
import { BN } from "@coral-xyz/anchor";
import { Action, Account, Utxo, AppUtxoConfig } from "../index";
type Asset = {
    sumIn: BN;
    sumOut: BN;
    asset: PublicKey;
};
export type Recipient = {
    account: Account;
    solAmount: BN;
    splAmount: BN;
    mint: PublicKey;
    appUtxo?: AppUtxoConfig;
};
export declare const getUtxoArrayAmount: (mint: PublicKey, inUtxos: Utxo[]) => BN;
export declare const getRecipientsAmount: (mint: PublicKey, recipients: Recipient[]) => BN;
export declare function createOutUtxos({ poseidon, inUtxos, outUtxos, publicMint, publicAmountSpl, publicAmountSol, relayerFee, changeUtxoAccount, action, appUtxo, numberMaxOutUtxos, assetLookupTable, verifierProgramLookupTable, separateSolUtxo, }: {
    inUtxos?: Utxo[];
    publicMint?: PublicKey;
    publicAmountSpl?: BN;
    publicAmountSol?: BN;
    relayerFee?: BN;
    poseidon: any;
    changeUtxoAccount: Account;
    outUtxos?: Utxo[];
    action: Action;
    appUtxo?: AppUtxoConfig;
    numberMaxOutUtxos: number;
    assetLookupTable: string[];
    verifierProgramLookupTable: string[];
    separateSolUtxo?: boolean;
}): Utxo[];
/**
 * @description Creates an array of UTXOs for each recipient based on their specified amounts and assets.
 *
 * @param recipients - Array of Recipient objects containing the recipient's account, SOL and SPL amounts, and mint.
 * @param poseidon - A Poseidon instance for hashing.
 *
 * @throws CreateUtxoError if a recipient has a mint defined but the SPL amount is undefined.
 * @returns An array of Utxos, one for each recipient.
 */
export declare function createRecipientUtxos({ recipients, poseidon, assetLookupTable, verifierProgramLookupTable, }: {
    recipients: Recipient[];
    poseidon: any;
    assetLookupTable: string[];
    verifierProgramLookupTable: string[];
}): Utxo[];
/**
 * @description Validates if the sum of input UTXOs for each asset is less than or equal to the sum of output UTXOs.
 *
 * @param assetPubkeys - Array of PublicKeys representing the asset public keys to be checked.
 * @param inUtxos - Array of input UTXOs containing the asset amounts being spent.
 * @param outUtxos - Array of output UTXOs containing the asset amounts being received.
 *
 * @throws Error if the sum of input UTXOs for an asset is less than the sum of output UTXOs.
 */
export declare function validateUtxoAmounts({ assetPubkeys, inUtxos, outUtxos, publicAmountSol, publicAmountSpl, action, }: {
    assetPubkeys: PublicKey[];
    inUtxos?: Utxo[];
    outUtxos: Utxo[];
    publicAmountSol?: BN;
    publicAmountSpl?: BN;
    action?: Action;
}): Asset[];
export {};
