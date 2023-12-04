import * as anchor from "@coral-xyz/anchor";
import {
  Utxo,
  hashAndTruncateToCircuit,
  PspTransactionInput,
  createProofInputs,
  getSystemProof,
  Account,
  SolMerkleTree,
  Relayer,
  setUndefinedPspCircuitInputsToZero,
  BN_0,
  TransactionParameters,
} from "@lightprotocol/zk.js";

import { SystemProgram, PublicKey } from "@solana/web3.js";
import { createPspTransaction, VoteWeightUtxoData } from "./index";
import { BN } from "@coral-xyz/anchor";

export type voteWeightCreationParamatersPda = {
  governingTokenMint: PublicKey;
  voteUtxoNumber: BN;
  publicMaxLockTime: BN;
};
export type createVoteWeightUtxoTransactionInput = {
  inUtxos: Utxo[];
  feeUtxo?: Utxo;
  voteWeightCreationParamatersPda: voteWeightCreationParamatersPda;
  idl: anchor.Idl;
  pspIdl: anchor.Idl;
  lookUpTables: { assetLookupTable: any; verifierProgramLookupTable: any };
  voter: Account;
  circuitPath: string;
  relayer: Relayer;
  solMerkleTree: SolMerkleTree;
  timeLocked: BN;
  voteUtxoNumber: BN;
  publicCurrentSlot: BN;
  voteWeightAmount: BN;
  verifierProgramId: PublicKey;
  voteWeightConfig: PublicKey;
  voteWeightProgramId: PublicKey;
};

export const createAndProveCreateVoteUtxoTransaction = async (
  createVoteWeightUtxoTransactionInput: createVoteWeightUtxoTransactionInput,
  poseidon: any
) => {
  const {
    inUtxos,
    feeUtxo,
    idl,
    pspIdl,
    lookUpTables,
    voter,
    circuitPath,
    relayer,
    solMerkleTree,
    timeLocked,
    voteUtxoNumber,
    voteWeightCreationParamatersPda,
    publicCurrentSlot,
    voteWeightAmount,
    verifierProgramId,
    voteWeightConfig,
    voteWeightProgramId,
  } = createVoteWeightUtxoTransactionInput;
  const amount = inUtxos
    .map((utxo) => utxo.amounts[0])
    .reduce((a, b) => a.add(b));
  if (amount.lt(voteWeightAmount)) {
    throw new Error(
      `inUtxos sum ${amount} must be greater than vote weight amount ${voteWeightAmount} `
    );
  }
  const rate = voteWeightAmount.div(timeLocked);
  const voteWeightUtxoData: VoteWeightUtxoData = {
    voteWeight: voteWeightAmount.mul(timeLocked),
    startSlot: publicCurrentSlot,
    releaseSlot: publicCurrentSlot.add(timeLocked),
    rate,
    voteLock: new BN(0),
    voteUtxoNumber,
    voteUtxoIdNonce: new BN(2), //nacl.randomBytes(31),
    voteWeightPspAddress: hashAndTruncateToCircuit(
      voteWeightProgramId.toBytes()
    ),
  };
  console.log(
    `\n\n ----------------  Creating vote weight utxo: vote weight ${voteWeightUtxoData.voteWeight} ---------------- \n\n`
  );
  // TODO: enable more than one utxo type in IDL and Utxo class
  const voteWeightUtxo = new Utxo({
    poseidon,
    assets: [SystemProgram.programId, SystemProgram.programId],
    publicKey: voter.pubkey,
    amounts: [voteWeightAmount, BN_0],
    appData: voteWeightUtxoData,
    appDataIdl: pspIdl,
    verifierAddress: verifierProgramId,
    assetLookupTable: lookUpTables.assetLookupTable,
  });
  if (feeUtxo) {
    inUtxos.push(feeUtxo);
  }
  const totalSolAmount = feeUtxo ? amount.add(feeUtxo.amounts[0]) : amount;
  const changeUtxo = new Utxo({
    poseidon,
    assets: [SystemProgram.programId],
    publicKey: voteWeightUtxo.publicKey,
    amounts: [totalSolAmount.sub(voteWeightAmount).sub(relayer.relayerFee)],
    assetLookupTable: lookUpTables.assetLookupTable,
  });
  // Remove accounts from Transaction it's the reason why
  // verifier state does not match it should not be created here
  const pspTransactionInput: PspTransactionInput = {
    proofInputs: {},
    path: circuitPath,
    verifierIdl: pspIdl,
    circuitName: "createVoteUtxo",
    checkedOutUtxos: [
      { utxoName: "createdVoteWeightUtxo", utxo: voteWeightUtxo },
    ],
    inUtxos,
    outUtxos: [changeUtxo],
    accounts: { voteWeightConfig, voteWeightProgram: voteWeightProgramId },
  };
  let transaction = await createPspTransaction(
    pspTransactionInput,
    poseidon,
    voter,
    relayer
  );
  transaction.verifierProgramId =
    TransactionParameters.getVerifierProgramId(idl);
  transaction.accounts.verifierState = PublicKey.findProgramAddressSync(
    [
      transaction.accounts.signingAddress.toBytes(),
      anchor.utils.bytes.utf8.encode("VERIFIER_STATE"),
    ],
    transaction.verifierProgramId
  )[0];
  // pspTransactionInput.accounts.verifierState = transaction.accounts.verifierState;
  // console.log("verifierProgramId", transaction.verifierProgramId.toBase58());
  // console.log("signingAddress", transaction.accounts.signingAddress.toBase58());
  // console.log("verifierState", transaction.accounts.verifierState.toBase58());

  const internalProofInputs = createProofInputs({
    poseidon,
    transaction,
    pspTransaction: pspTransactionInput,
    account: voter,
    solMerkleTree,
  });
  const proofInputs = {
    ...internalProofInputs,
    // overwriting the publicAppVerifier because we are using cpi to verify the vote weight utxo creation proof
    publicAppVerifier: hashAndTruncateToCircuit(verifierProgramId.toBytes()),
    publicPspAddress: hashAndTruncateToCircuit(voteWeightProgramId.toBytes()),
    governingTokenMint: hashAndTruncateToCircuit(
      voteWeightCreationParamatersPda.governingTokenMint.toBytes()
    ),
    publicMaxLockTime: voteWeightCreationParamatersPda.publicMaxLockTime,
    publicCurrentSlot,
    publicVoteUtxoNumber: voteWeightCreationParamatersPda.voteUtxoNumber,
    publicGoverningTokenMint: hashAndTruncateToCircuit(
      voteWeightCreationParamatersPda.governingTokenMint.toBytes()
    ),
    create: new BN(1),
  };
  console.time("SystemProof");

  const systemProof = await getSystemProof({
    account: voter,
    transaction,
    systemProofInputs: proofInputs,
  });
  console.timeEnd("SystemProof");

  const completePspProofInputs = setUndefinedPspCircuitInputsToZero(
    proofInputs,
    pspIdl,
    pspTransactionInput.circuitName
  );
  console.time("PspProof");
  const pspProof = await voter.getProofInternal(
    pspTransactionInput.path,
    pspTransactionInput,
    completePspProofInputs,
    false
  );
  pspTransactionInput.verifierIdl = idl;
  console.timeEnd("PspProof");
  return { systemProof, pspProof, transaction, pspTransactionInput };
};
