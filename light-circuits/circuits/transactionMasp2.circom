pragma circom 2.0.0;
include "./transaction_masp.circom";

// 2 in 2 out 3 assets (min to do a swap)
component main {public [root,inputNullifier, outputCommitment,publicAmount,extDataHash,feeAmount,mintPubkey]} = TransactionAccount(18, 2, 2, 0, 0, 1, 3, 3, 3);
