/// TODO: extract wallet into its own npm package
import {
    Commitment,
    Connection,
    Keypair,
    VersionedTransaction,
    sendAndConfirmTransaction,
} from '@solana/web3.js';
import { PublicKey, Transaction } from '@solana/web3.js';
import nacl from 'tweetnacl';
const { sign } = nacl;

export type InclusionProofPublicInputs = {
    root: string;
    leaf: string;
};
export type InclusionProofPrivateInputs = {
    merkleProof: string[];
    leaf: string;
    leafIndex: string;
};

/// On the system level, we're proving simple inclusion proofs in a
/// state tree, for each utxo used as input into a transaction.
export type InclusionProofInputs = (InclusionProofPublicInputs &
    InclusionProofPrivateInputs)[];

/// Mock Solana web3 library
export class Wallet {
    _publicKey: PublicKey;
    _keypair: Keypair;
    _connection: Connection;
    _url: string;
    _commitment: Commitment;

    constructor(keypair: Keypair, url: string, commitment: Commitment) {
        this._publicKey = keypair.publicKey;
        this._keypair = keypair;
        this._connection = new Connection(url);
        this._url = url;
        this._commitment = commitment;
    }

    signTransaction = async (tx: any): Promise<any> => {
        await tx.sign([this._keypair!]);
        return tx;
    };

    sendTransaction = async (
        transaction: VersionedTransaction,
    ): Promise<string> => {
        const signature = await this._connection.sendTransaction(transaction);
        return signature;
    };

    signAllTransactions = async <T extends Transaction | VersionedTransaction>(
        transactions: T[],
    ): Promise<T[]> => {
        const signedTxs = await Promise.all(
            transactions.map(async tx => {
                return await this.signTransaction(tx);
            }),
        );
        return signedTxs;
    };

    signMessage = async (message: Uint8Array): Promise<Uint8Array> => {
        return sign.detached(message, this._keypair.secretKey);
    };

    sendAndConfirmTransaction = async (
        transaction: Transaction,
        signers = [],
    ): Promise<any> => {
        const response = await sendAndConfirmTransaction(
            this._connection,
            transaction,
            [this._keypair, ...signers],
            {
                commitment: this._commitment,
            },
        );
        return response;
    };
}
