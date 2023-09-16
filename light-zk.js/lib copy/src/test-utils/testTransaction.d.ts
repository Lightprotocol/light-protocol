/// <reference types="bn.js" />
import { Account, TransactionParameters, Provider, IDL_MERKLE_TREE_PROGRAM } from "../index";
import { BN, Program } from "@coral-xyz/anchor";
export declare class TestTransaction {
    testValues?: {
        recipientBalancePriorTx?: BN;
        relayerRecipientAccountBalancePriorLastTx?: BN;
        txIntegrityHash?: BN;
        senderFeeBalancePriorTx?: BN;
        recipientFeeBalancePriorTx?: BN;
        is_token?: boolean;
    };
    params: TransactionParameters;
    provider: Provider;
    merkleTreeProgram?: Program<typeof IDL_MERKLE_TREE_PROGRAM>;
    appParams?: any;
    constructor({ txParams, provider, appParams, }: {
        txParams: TransactionParameters;
        appParams?: any;
        provider: Provider;
    });
    getTestValues(): Promise<void>;
    checkBalances(transactionInputs: any, remainingAccounts: any, proofInput: any, account?: Account): Promise<void>;
}
