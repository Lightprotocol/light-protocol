pragma circom 2.0.0;
include "./transaction_app.circom";

// 2 in 2 out 3 assets (min to do a swap)
component main {public [root,inputNullifier, outputCommitment,publicAmountSpl,txIntergrityHash,publicAmountSol,publicMintPubkey]} = TransactionAccount(0,0, 18, 2, 2, 24603683191960664281975569809895794547840992286820815015841170051925534051, 0, 1, 3, 2, 2, 1);
