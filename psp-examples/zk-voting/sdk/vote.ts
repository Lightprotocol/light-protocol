import * as anchor from "@coral-xyz/anchor";
import {
  Utxo,
  TransactionParameters,
  PspTransactionInput,
  createProofInputs,
  getSystemProof,
  circuitlibjs,
  Account,
  SolMerkleTree,
  Relayer,
  BN_0,
  BN_1,
} from "@lightprotocol/zk.js";
const { ElGamalUtils } = circuitlibjs;
const { pointToStringArray, coordinatesToExtPoint } = ElGamalUtils;
import { SystemProgram, PublicKey } from "@solana/web3.js";
import {
  encrypt,
  PublicKey as ElGamalPublicKey,
  generateRandomSalt,
} from "@lightprotocol/circuit-lib.js";
import { ExtPointType } from "@noble/curves/abstract/edwards";

import { BN } from "@coral-xyz/anchor";
import { IDL } from "../target/types/private_voting";
import { utils } from "@project-serum/anchor";
import { createPspTransaction } from "./utils";

export type VoteParameters = {
  governingTokenMint: PublicKey;
  startVotingAt: BN;
  votingCompletedAt: BN;
  maxVoteWeight: BN;
  voteThreshold: BN;
  name: string;
  vetoVoteWeight: BN;
  elGamalPublicKey: ElGamalPublicKey;
};

export type VoteWeightUtxoData = {
  voteWeight: BN;
  startSlot: BN;
  releaseSlot: BN;
  rate: BN;
  voteLock: BN;
  voteUtxoNumber: BN;
  voteUtxoIdNonce?: BN;
  voteWeightPspAddress: BN;
};
export type VoteTransactionInput = {
  voteWeightUtxo: Utxo;
  feeUtxo: Utxo;
  voteParameters: VoteParameters;
  idl: anchor.Idl;
  lookUpTables: { assetLookupTable: any; verifierProgramLookupTable: any };
  proofInputs: {
    currentSlot: BN;
    publicElGamalPublicKeyX: BN;
    publicElGamalPublicKeyY: BN;
    publicOldVoteWeightNoEmphemeralKeyX: BN;
    publicOldVoteWeightNoEmphemeralKeyY: BN;
    publicOldVoteWeightYesEmphemeralKeyX: BN;
    publicOldVoteWeightYesEmphemeralKeyY: BN;
    publicOldVoteWeightNoCiphertextX: BN;
    publicOldVoteWeightNoCiphertextY: BN;
    publicOldVoteWeightYesCiphertextX: BN;
    publicOldVoteWeightYesCiphertextY: BN;
  };
  voter: Account;
  circuitPath: string;
  relayer: Relayer;
  solMerkleTree: SolMerkleTree;
  voteYes: boolean;
};

export const createAndProveVoteTransaction = async (
  voteTransactionInput: VoteTransactionInput,
  poseidon: any
) => {
  const {
    voteWeightUtxo,
    voteParameters,
    idl,
    lookUpTables,
    feeUtxo,
    proofInputs,
    voter,
    circuitPath,
    relayer,
    solMerkleTree,
    voteYes,
  } = voteTransactionInput;
  // create locked vote weight utxo
  const lockedVoteWeightUtxoData: VoteWeightUtxoData = {
    ...voteWeightUtxo.appData,
    voteLock: voteParameters.votingCompletedAt,
  };
  // TODO: create a function which is the equivalent to the checks in circuits ideally derives those from IDL
  // TODO: create outUtxo <name> { type: , equalsUtxo: <utxoName>, ...} defined checks overwrite utxo checks
  const lockedVoteWeightUtxo = new Utxo({
    poseidon,
    assets: voteWeightUtxo.assets,
    publicKey: voteWeightUtxo.publicKey,
    amounts: voteWeightUtxo.amounts,
    appData: lockedVoteWeightUtxoData,
    appDataIdl: voteWeightUtxo.appDataIdl,
    verifierAddress: voteWeightUtxo.verifierAddress,
    assetLookupTable: lookUpTables.assetLookupTable,
  });

  const changeUtxo = new Utxo({
    poseidon,
    assets: [SystemProgram.programId],
    publicKey: voteWeightUtxo.publicKey,
    amounts: [feeUtxo.amounts[0].sub(relayer.relayerFee)],
    verifierAddress: TransactionParameters.getVerifierProgramId(idl),
    assetLookupTable: lookUpTables.assetLookupTable,
  });

  const publicVoteId = new BN(utils.bytes.utf8.encode("publicVoteId"));

  const nullifier = new BN(
    poseidon.F.toString(
      poseidon([voteWeightUtxo.appData.voteUtxoNumber, publicVoteId])
    )
  );

  const publicOldVoteWeightYesEmphemeralKey = coordinatesToExtPoint<BigInt>(
    BigInt(proofInputs.publicOldVoteWeightYesEmphemeralKeyX.toString()),
    BigInt(proofInputs.publicOldVoteWeightYesEmphemeralKeyY.toString())
  );
  const publicOldVoteWeightYes: ExtPointType = coordinatesToExtPoint<BigInt>(
    BigInt(proofInputs.publicOldVoteWeightYesCiphertextX.toString()),
    BigInt(proofInputs.publicOldVoteWeightYesCiphertextY.toString())
  );
  const publicOldVoteWeightNoEmphemeralKey: ExtPointType =
    coordinatesToExtPoint<BigInt>(
      BigInt(proofInputs.publicOldVoteWeightNoEmphemeralKeyX.toString()),
      BigInt(proofInputs.publicOldVoteWeightNoEmphemeralKeyY.toString())
    );
  const publicOldVoteWeightNo: ExtPointType = coordinatesToExtPoint<BigInt>(
    BigInt(proofInputs.publicOldVoteWeightNoCiphertextX.toString()),
    BigInt(proofInputs.publicOldVoteWeightNoCiphertextY.toString())
  );
  // encrypt vote weight
  // - vote weight
  // - el gamal public key
  const nonceVoteCiphertext = generateRandomSalt();
  const nonceZeroCiphertext = generateRandomSalt();
  const { ephemeralKey, ciphertext } = encrypt(
    voteParameters.elGamalPublicKey,
    BigInt(voteWeightUtxo.appData.voteWeight.toString()),
    nonceVoteCiphertext
  );

  const { ephemeralKey: zeroEphemeralKey, ciphertext: zeroCiphertext } =
    encrypt(voteParameters.elGamalPublicKey, BigInt(0), nonceZeroCiphertext);
  const addedEmphemeralKey = ephemeralKey.add(
    voteYes
      ? publicOldVoteWeightYesEmphemeralKey
      : publicOldVoteWeightNoEmphemeralKey
  );
  const addedCiphertext = ciphertext.add(
    voteYes ? publicOldVoteWeightYes : publicOldVoteWeightNo
  );
  const addedZeroEmphemeralKey = zeroEphemeralKey.add(
    !voteYes
      ? publicOldVoteWeightYesEmphemeralKey
      : publicOldVoteWeightNoEmphemeralKey
  );
  const addedZeroCiphertext = zeroCiphertext.add(
    !voteYes ? publicOldVoteWeightYes : publicOldVoteWeightNo
  );

  const addedEmphemeralKeyString = pointToStringArray(addedEmphemeralKey);
  const addedCiphertextString = pointToStringArray(addedCiphertext);
  const addedZeroEmphemeralKeyString = pointToStringArray(
    addedZeroEmphemeralKey
  );
  const addedZeroCiphertextString = pointToStringArray(addedZeroCiphertext);

  const publicOldVoteWeightYesEmphemeralKeyString = pointToStringArray(
    publicOldVoteWeightYesEmphemeralKey
  );
  const publicOldVoteWeightYesString = pointToStringArray(
    publicOldVoteWeightYes
  );
  const publicOldVoteWeightNoEmphemeralKeyString = pointToStringArray(
    publicOldVoteWeightNoEmphemeralKey
  );
  const publicOldVoteWeightNoString = pointToStringArray(publicOldVoteWeightNo);

  const pspTransactionInput: PspTransactionInput = {
    proofInputs: {
      publicMint: BN_0,
      publicVoteWeightYesX: voteYes
        ? addedCiphertextString[0]
        : addedZeroCiphertextString[0], // TODO: do adds
      publicVoteWeightYesY: voteYes
        ? addedCiphertextString[1]
        : addedZeroCiphertextString[1],
      publicVoteWeightYesEmphemeralKeyX: voteYes
        ? addedEmphemeralKeyString[0]
        : addedZeroEmphemeralKeyString[0],
      publicVoteWeightYesEmphemeralKeyY: voteYes
        ? addedEmphemeralKeyString[1]
        : addedZeroEmphemeralKeyString[1],
      publicVoteWeightNoX: !voteYes
        ? addedCiphertextString[0]
        : addedZeroCiphertextString[0],
      publicVoteWeightNoY: !voteYes
        ? addedCiphertextString[1]
        : addedZeroCiphertextString[1],
      publicVoteWeightNoEmphemeralKeyX: !voteYes
        ? addedEmphemeralKeyString[0]
        : addedZeroEmphemeralKeyString[0],
      publicVoteWeightNoEmphemeralKeyY: !voteYes
        ? addedEmphemeralKeyString[1]
        : addedZeroEmphemeralKeyString[1],
      nonceVoteCiphertext,
      nonceZeroCiphertext,
      publicVoteId,
      publicVoteWeightPspAddress: voteWeightUtxo.appData.voteWeightPspAddress,
      voteWeightNullifier: nullifier,
      publicVoteEnd: voteParameters.votingCompletedAt,
      choiceIsYes: voteYes ? BN_1 : BN_0,
      ...proofInputs,
      publicOldVoteWeightYesX: new BN(publicOldVoteWeightYesString[0]),
      publicOldVoteWeightYesY: new BN(publicOldVoteWeightYesString[1]),
      publicOldVoteWeightYesEmphemeralKeyX: new BN(
        publicOldVoteWeightYesEmphemeralKeyString[0]
      ),
      publicOldVoteWeightYesEmphemeralKeyY: new BN(
        publicOldVoteWeightYesEmphemeralKeyString[1]
      ),
      publicOldVoteWeightNoX: new BN(publicOldVoteWeightNoString[0]),
      publicOldVoteWeightNoY: new BN(publicOldVoteWeightNoString[1]),
      publicOldVoteWeightNoEmphemeralKeyX: new BN(
        publicOldVoteWeightNoEmphemeralKeyString[0]
      ),
      publicOldVoteWeightNoEmphemeralKeyY: new BN(
        publicOldVoteWeightNoEmphemeralKeyString[1]
      ),
    },
    path: circuitPath,
    verifierIdl: IDL,
    circuitName: "privateVoting",
    checkedInUtxos: [{ utxoName: "voteWeightUtxo", utxo: voteWeightUtxo }],
    checkedOutUtxos: [
      { utxoName: "lockedVoteWeightUtxo", utxo: lockedVoteWeightUtxo },
    ],
    inUtxos: [feeUtxo],
    outUtxos: [changeUtxo],
    accounts: {},
  };
  let transaction = await createPspTransaction(
    pspTransactionInput,
    poseidon,
    voter,
    relayer
  );

  const internalProofInputs = createProofInputs({
    poseidon,
    transaction,
    pspTransaction: pspTransactionInput,
    account: voter,
    solMerkleTree,
  });
  console.time("SystemProof");

  const systemProof = await getSystemProof({
    account: voter,
    transaction,
    systemProofInputs: internalProofInputs,
  });
  console.timeEnd("SystemProof");

  console.time("PspProof");
  const pspProof = await voter.getProofInternal(
    pspTransactionInput.path,
    pspTransactionInput,
    internalProofInputs,
    false
  );
  console.timeEnd("PspProof");
  return { pspProof, systemProof, transaction, pspTransactionInput };
};
