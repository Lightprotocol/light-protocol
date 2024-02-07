pragma circom 2.0.0;

include "./transaction.circom";

// 2 in 2 out 3 assets (min to do a swap)
component main {
	public [
		publicStateRoot,
		publicInUtxoHash,
		publicOutUtxoHash,
		publicAmountSpl,
		publicDataHash,
		publicAmountSol,
		publicMintPublicKey,
		publicInUtxoDataHash,
		publicNewAddress
	]
} = PublicProgramTransaction(
	22,
	2,
	2,
	6686672797465227418401714772753289406522066866583537086457438811846503839916,
	0,
	1,
	3,
	2,
	2,
	2
);
