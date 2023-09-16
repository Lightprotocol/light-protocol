/// <reference types="bn.js" />
import { Connection, Keypair, PublicKey } from "@solana/web3.js";
import { BN } from "@coral-xyz/anchor";
import { Relayer, RelayerSendTransactionsResponse } from "../relayer";
import { Provider } from "../wallet";
import { ParsedIndexedTransaction } from "../types";
export declare class TestRelayer extends Relayer {
    indexedTransactions: ParsedIndexedTransaction[];
    relayerKeypair: Keypair;
    constructor({ relayerPubkey, relayerRecipientSol, relayerFee, highRelayerFee, payer, }: {
        relayerPubkey: PublicKey;
        relayerRecipientSol?: PublicKey;
        relayerFee: BN;
        highRelayerFee?: BN;
        payer: Keypair;
    });
    updateMerkleTree(provider: Provider): Promise<any>;
    sendTransactions(instructions: any[], provider: Provider): Promise<RelayerSendTransactionsResponse>;
    /**
     * Indexes light transactions by:
     * - getting all signatures the merkle tree was involved in
     * - trying to extract and parse event cpi for every signature's transaction
     * - if there are indexed transactions already in the relayer object only transactions after the last indexed event are indexed
     * @param connection
     * @returns
     */
    getIndexedTransactions(connection: Connection): Promise<ParsedIndexedTransaction[]>;
}
