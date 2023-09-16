/// <reference types="bn.js" />
import { Connection, PublicKey, RpcResponseAndContext, SignatureResult } from "@solana/web3.js";
import { BN } from "@coral-xyz/anchor";
import { Provider, SendVersionedTransactionsResult, ParsedIndexedTransaction } from "./index";
export type RelayerSendTransactionsResponse = SendVersionedTransactionsResult & {
    transactionStatus: string;
    rpcResponse?: RpcResponseAndContext<SignatureResult>;
};
export declare class Relayer {
    accounts: {
        relayerPubkey: PublicKey;
        relayerRecipientSol: PublicKey;
    };
    relayerFee: BN;
    highRelayerFee: BN;
    indexedTransactions: ParsedIndexedTransaction[];
    url: string;
    /**
     *
     * @param relayerPubkey Signs the transaction
     * @param relayerRecipientSol Recipient account for SOL fees
     * @param relayerFee Fee amount
     */
    constructor(relayerPubkey: PublicKey, relayerRecipientSol?: PublicKey, relayerFee?: BN, highRelayerFee?: BN, url?: string);
    updateMerkleTree(_provider: Provider): Promise<import("axios").AxiosResponse<any, any>>;
    sendTransactions(instructions: any[], _provider: Provider): Promise<RelayerSendTransactionsResponse>;
    getRelayerFee(ataCreationFee?: boolean): BN;
    getIndexedTransactions(_connection: Connection): Promise<ParsedIndexedTransaction[]>;
}
//# sourceMappingURL=relayer.d.ts.map