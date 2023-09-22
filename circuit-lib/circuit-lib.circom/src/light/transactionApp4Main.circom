pragma circom 2.0.0;

include "./transaction_app.circom";

// 2 in 2 out 3 assets (min to do a swap)
component main {
	public [
		root,
		inputNullifier,
		outputCommitment,
		publicAmountSpl,
		txIntegrityHash,
		publicAmountSol,
		publicMintPubkey,
		publicAppVerifier,
		transactionHash
	]
} = TransactionAccount(
	18,
	4,
	4,
	184598798020101492503359154328231866914977581098629757339001774613643340069,
	0,
	1,
	3,
	2,
	2
);
