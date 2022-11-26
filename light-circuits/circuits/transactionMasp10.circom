pragma circom 2.0.0;
include "./transaction_masp.circom";

component main {public [root,inputNullifier, outputCommitment,publicAmount,extDataHash,feeAmount,mintPubkey]} = TransactionAccount(18, 10, 2, 0, 0, 1, 3, 3, 3);
