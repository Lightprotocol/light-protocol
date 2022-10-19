pragma circom 2.0.0;
include "./transaction_masp.circom";

// 2 in 2 out 3 assets (min to do a swap)
component main {public [root,inputNullifier, outputCommitment,publicAmount,extDataHash,feeAmount,mintPubkey]} = TransactionAccount(18, 10, 2, 14522046728041339886521211779101644712859239303505368468566383402165481390632, 0, 0, 3);
/*template instances: 295
non-linear constraints: 14034
linear constraints: 0
public inputs: 9
public outputs: 0
private inputs: 77
private outputs: 0
wires: 14085
labels: 43604
*/
/*
with 3 assets per utxo
non-linear constraints: 15118
linear constraints: 0
public inputs: 9
public outputs: 0
private inputs: 101
private outputs: 0
wires: 15169
labels: 47412
*/
