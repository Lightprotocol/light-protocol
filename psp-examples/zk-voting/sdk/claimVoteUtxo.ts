import * as anchor from "@coral-xyz/anchor";
import {
  Utxo,
  hashAndTruncateToCircuit,
  PspTransactionInput,
  createProofInputs,
  getSystemProof,
  circuitlibjs,
  Account,
  SolMerkleTree,
  Relayer,
  setUndefinedPspCircuitInputsToZero,
  TransactionParameters,
} from "@lightprotocol/zk.js";

import { SystemProgram, PublicKey } from "@solana/web3.js";

import { createPspTransaction } from "./index";

import { BN } from "@coral-xyz/anchor";
import { voteWeightCreationParamatersPda } from "./index";

export type claimVoteWeightUtxoTransactionInput = {
  voteWeightUtxo: Utxo;
  feeUtxo?: Utxo;
  voteWeightCreationParamatersPda: voteWeightCreationParamatersPda;
  idl: anchor.Idl;
  pspIdl: anchor.Idl;
  lookUpTables: { assetLookupTable: any; verifierProgramLookupTable: any };
  voter: Account;
  circuitPath: string;
  relayer: Relayer;
  solMerkleTree: SolMerkleTree;
  publicCurrentSlot: BN;
  verifierProgramId: PublicKey;
  voteWeightConfig: PublicKey;
  voteWeightProgramId: PublicKey;
};
export const createAndProveClaimVoteUtxoTransaction = async (
  createVoteWeightUtxoTransactionInput: claimVoteWeightUtxoTransactionInput,
  poseidon: any
) => {
  const {
    voteWeightUtxo,
    feeUtxo,
    idl,
    pspIdl,
    lookUpTables,
    voter,
    circuitPath,
    relayer,
    solMerkleTree,
    voteWeightCreationParamatersPda,
    publicCurrentSlot,
    verifierProgramId,
    voteWeightConfig,
    voteWeightProgramId,
  } = createVoteWeightUtxoTransactionInput;

  console.log(
    `\n\n ----------------  Claiming vote weight utxo: vote weight ${voteWeightUtxo.appData.voteWeight} ---------------- \n\n`
  );
  // TODO: enable more than one utxo type in IDL and Utxo class
  const claimUtxo = new Utxo({
    poseidon,
    assets: [SystemProgram.programId],
    publicKey: voter.pubkey,
    amounts: [voteWeightUtxo.amounts[0]],
    assetLookupTable: lookUpTables.assetLookupTable,
  });

  const changeUtxo = new Utxo({
    poseidon,
    assets: [SystemProgram.programId],
    publicKey: voteWeightUtxo.publicKey,
    amounts: [feeUtxo.amounts[0].sub(relayer.relayerFee)],
    assetLookupTable: lookUpTables.assetLookupTable,
  });

  const pspTransactionInput: PspTransactionInput = {
    proofInputs: {
      // publicPspAddress: hashAndTruncateToCircuit(voteWeightProgramId.toBytes()),
    },
    path: circuitPath,
    verifierIdl: pspIdl,
    circuitName: "createVoteUtxo",
    checkedInUtxos: [
      { utxoName: "claimVoteWeightInUtxo", utxo: voteWeightUtxo },
    ],
    inUtxos: [feeUtxo],
    outUtxos: [claimUtxo, changeUtxo],
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
    claim: new BN(1),
    create: new BN(0),
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
  console.timeEnd("PspProof");
  pspTransactionInput.verifierIdl = idl;
  return { systemProof, pspProof, transaction, pspTransactionInput };
};
