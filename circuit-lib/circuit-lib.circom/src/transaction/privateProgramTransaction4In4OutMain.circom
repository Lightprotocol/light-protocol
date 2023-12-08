pragma circom 2.1.4;

include "./transaction.circom";

// 2 in 2 out 3 assets (min to do a swap)
component main {
	public [
		publicStateRoot,
		publicNullifierRoot,
		publicNullifier,
		publicOutUtxoHash,
		publicAmountSpl,
		publicDataHash,
		publicAmountSol,
		publicMintPublicKey,
		publicProgramId,
		publicTransactionHash
	]
} = PrivateProgramTransaction(
	22,
	4,
	4,
	6686672797465227418401714772753289406522066866583537086457438811846503839916,
	0,
	1,
	3,
	2,
	2,
	4
);
