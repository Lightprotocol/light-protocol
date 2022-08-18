pragma circom 2.0.0;
include "./transaction_masp.circom";
// include "./transaction_masp_2_assets_utxo.circom";

// zeroLeaf = Poseidon(zero, zero)
// default `zero` value is keccak256("tornado") % FIELD_SIZE = 21663839004416932945382355908790599225266501822907911457504978515578255421292
//11111111111111111111111111111111 systemProgram -> mint for sol
// component main {public [root,inputNullifier, outputCommitment,publicAmount,extDataHash,feeAmount,mintPubkey]} = Transaction(18, 2, 2, 14522046728041339886521211779101644712859239303505368468566383402165481390632, 0, 0, 3);
// 2.5s proofgen with 2 inputs masp, and feeAmount in every utxo
/*on-linear constraints: 12174
linear constraints: 0
public inputs: 9
public outputs: 0
private inputs: 83
private outputs: 0
wires: 12203
labels: 39890
*/
// performance diff 88683 vs 93743 constraints -> it's probably worth keeping the general hash

/*
non-linear constraints: 12138
linear constraints: 0
public inputs: 9
public outputs: 0
private inputs: 73
private outputs: 0
wires: 12189
labels: 39848
*/
// component main {public [root,inputNullifier, outputCommitment,publicAmount,extDataHash,feeAmount,mintPubkey]} = TransactionAccount(18, 16,2, 14522046728041339886521211779101644712859239303505368468566383402165481390632, 0, 0, 3);

// 2 in 2 out 5 assets
// component main {public [root,inputNullifier, outputCommitment,publicAmount,extDataHash,feeAmount,mintPubkey]} = TransactionAccount(18, 2,2, 14522046728041339886521211779101644712859239303505368468566383402165481390632, 0, 0, 5);
/*
template instances: 295
non-linear constraints: 15274
linear constraints: 0
public inputs: 9
public outputs: 0
private inputs: 95
private outputs: 0
wires: 15327
labels: 46298
*/
// 2 in 2 out 3 assets (min to do a swap)
component main {public [root,inputNullifier, outputCommitment,publicAmount,extDataHash,feeAmount,mintPubkey]} = TransactionAccount(18, 2, 2, 14522046728041339886521211779101644712859239303505368468566383402165481390632, 0, 0, 3);
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
