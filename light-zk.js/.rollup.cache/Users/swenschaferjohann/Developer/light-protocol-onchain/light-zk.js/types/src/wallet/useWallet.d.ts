import { Commitment, Keypair } from "@solana/web3.js";
import { PublicKey, Transaction } from "@solana/web3.js";
export declare const useWallet: (keypair: Keypair, url?: string, isNodeWallet?: boolean, commitment?: Commitment) => {
    publicKey: PublicKey;
    sendAndConfirmTransaction: (transaction: Transaction, signers?: any[]) => Promise<any>;
    signMessage: (message: Uint8Array) => Promise<Uint8Array>;
    signTransaction: (tx: any) => Promise<any>;
    signAllTransactions: (transactions: Transaction[]) => Promise<Transaction[]>;
    isNodeWallet: boolean;
};
//# sourceMappingURL=useWallet.d.ts.map