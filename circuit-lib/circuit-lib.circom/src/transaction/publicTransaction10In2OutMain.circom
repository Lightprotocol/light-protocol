pragma circom 2.1.4;

include "./transaction.circom";

component main {
	public [
		publicStateRoot,
		publicInUtxoHash,
		publicOutUtxoHash,
		publicDataHash
	]
} = PublicTransaction(
	18,
	8,
	2,
	184598798020101492503359154328231866914977581098629757339001774613643340069,
	0,
	1,
	3,
	2,
	2,
	8
);
