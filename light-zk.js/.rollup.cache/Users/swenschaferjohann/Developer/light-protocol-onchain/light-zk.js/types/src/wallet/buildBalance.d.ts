/// <reference types="bn.js" />
import { Utxo } from "../utxo";
import * as anchor from "@coral-xyz/anchor";
import { BN } from "@coral-xyz/anchor";
import { Connection, PublicKey } from "@solana/web3.js";
import { Account } from "../account";
import { TokenData } from "../index";
export type Balance = {
    tokenBalances: Map<string, TokenUtxoBalance>;
    programBalances: Map<string, ProgramUtxoBalance>;
    nftBalances: Map<string, TokenUtxoBalance>;
    totalSolBalance: BN;
};
export type InboxBalance = Balance & {
    numberInboxUtxos: number;
};
type VariableType = "utxos" | "committedUtxos" | "spentUtxos";
export declare class TokenUtxoBalance {
    tokenData: TokenData;
    totalBalanceSpl: BN;
    totalBalanceSol: BN;
    utxos: Map<string, Utxo>;
    committedUtxos: Map<string, Utxo>;
    spentUtxos: Map<string, Utxo>;
    constructor(tokenData: TokenData);
    static initSol(): TokenUtxoBalance;
    addUtxo(commitment: string, utxo: Utxo, attribute: VariableType): boolean;
    moveToSpentUtxos(commitment: string): void;
}
export declare class ProgramUtxoBalance {
    programAddress: PublicKey;
    programUtxoIdl: anchor.Idl;
    tokenBalances: Map<string, TokenUtxoBalance>;
    constructor(programAddress: PublicKey, programUtxoIdl: anchor.Idl);
    addUtxo(commitment: string, utxo: Utxo, attribute: VariableType): boolean;
}
export declare class ProgramBalance extends TokenUtxoBalance {
    programAddress: PublicKey;
    programUtxoIdl: anchor.Idl;
    constructor(tokenData: TokenData, programAddress: PublicKey, programUtxoIdl: anchor.Idl);
    addProgramUtxo(commitment: string, utxo: Utxo, attribute: VariableType): boolean;
}
export declare function decryptAddUtxoToBalance({ account, encBytes, index, commitment, poseidon, connection, balance, merkleTreePdaPublicKey, leftLeaf, aes, verifierProgramLookupTable, assetLookupTable, }: {
    encBytes: Uint8Array;
    index: number;
    commitment: Uint8Array;
    account: Account;
    merkleTreePdaPublicKey: PublicKey;
    poseidon: any;
    connection: Connection;
    balance: Balance;
    leftLeaf: Uint8Array;
    aes: boolean;
    verifierProgramLookupTable: string[];
    assetLookupTable: string[];
}): Promise<void>;
export {};
//# sourceMappingURL=buildBalance.d.ts.map