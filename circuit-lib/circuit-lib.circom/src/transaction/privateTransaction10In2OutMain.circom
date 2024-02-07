pragma circom 2.1.4;

include "./transaction.circom";

component main {
	public [
		publicStateRoot,
		publicNullifierRoot,
		publicNullifier,
		publicOutUtxoHash,
		publicAmountSpl,
		publicDataHash,
		publicAmountSol,
		publicMintPublicKey
	]
} = PrivateTransaction(
	22,
	10,
	2,
	6686672797465227418401714772753289406522066866583537086457438811846503839916,
	0,
	1,
	3,
	2,
	2,
	10
);
