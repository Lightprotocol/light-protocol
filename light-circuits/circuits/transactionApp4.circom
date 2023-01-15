pragma circom 2.0.0;
include "./transaction_app.circom";

// 2 in 2 out 3 assets (min to do a swap)
component main {public [root,inputNullifier, outputCommitment,publicAmount,extDataHash,feeAmount,mintPubkey, verifier, connectingHash]} = TransactionAccount(18, 4, 4, 0, 0, 1, 3, 2, 2);
