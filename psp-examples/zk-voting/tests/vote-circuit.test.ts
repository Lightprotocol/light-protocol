import * as anchor from "@coral-xyz/anchor";
import {
  Utxo,
  TransactionParameters,
  Action,
  TestRelayer,
  Provider,
  hashAndTruncateToCircuit,
  PspTransactionInput,
  MerkleTreeConfig,
  IDL_LIGHT_PSP4IN4OUT_APP_STORAGE,
  getVerifierStatePda,
  createProofInputs,
  getSystemProof,
  circuitlibjs,
  Account,
  SolMerkleTree,
  Relayer,
  BN_0,
  BN_1,
  FIELD_SIZE,
} from "@lightprotocol/zk.js";
const { MerkleTree, ElGamalUtils } = circuitlibjs;
const {
  pointToStringArray,
  stringifyBigInts,
  toBigIntArray,
  coordinatesToExtPoint,
} = ElGamalUtils;
import { SystemProgram, PublicKey, Keypair } from "@solana/web3.js";
import {
  encrypt,
  PublicKey as ElGamalPublicKey,
  generateKeypair,
  generateRandomSalt,
  babyjubjubExt,
} from "@lightprotocol/circuit-lib.js";
import { ExtPointType } from "@noble/curves/abstract/edwards";

import { buildPoseidonOpt } from "circomlibjs";
import { BN } from "@coral-xyz/anchor";
import { IDL, PrivateVoting } from "../target/types/private_voting";
import { utils } from "@project-serum/anchor";
const path = require("path");

const verifierProgramId = new PublicKey(
  "Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS"
);
var POSEIDON;

const RPC_URL = "http://127.0.0.1:8899";

/**
 * 1. create proposal
 * 2. create vote utxo
 * 3. init vote
 * 4. vote
 */
describe("Test private-voting", () => {
  process.env.ANCHOR_PROVIDER_URL = RPC_URL;
  process.env.ANCHOR_WALLET = process.env.HOME + "/.config/solana/id.json";

  // Configure the client to use the local cluster.
  let proposerKeypair: Keypair,
    voterKeypair: Keypair,
    voteProgram: anchor.Program<PrivateVoting> = anchor.workspace.PrivateVoting;
  let proposalPda: PublicKey;
  let voter: Account, lightProvider: Provider, localTestRelayer: TestRelayer;

  before(async () => {
    POSEIDON = await buildPoseidonOpt();
    proposerKeypair = Keypair.generate();
    voterKeypair = Keypair.generate();

    voteProgram = new anchor.Program<PrivateVoting>(IDL, verifierProgramId);
    proposalPda = PublicKey.findProgramAddressSync(
      [
        proposerKeypair.publicKey.toBuffer(),
        utils.bytes.utf8.encode("MockProposalV2"),
      ],
      verifierProgramId
    )[0];

    const relayerWallet = Keypair.generate();

    localTestRelayer = new TestRelayer({
      relayerPubkey: relayerWallet.publicKey,
      relayerRecipientSol: relayerWallet.publicKey,
      relayerFee: new BN(100000),
      payer: relayerWallet,
    });

    lightProvider = await Provider.loadMock();

    voter = Account.createFromSolanaKeypair(POSEIDON, voterKeypair);
  });

  it(" test vote circuit ", async () => {
    const voteAdminElGamalSecretKey = generateKeypair();

    const voteParameters: VoteParameters = {
      governingTokenMint: SystemProgram.programId,
      startVotingAt: new BN(0),
      votingCompletedAt: new BN(1000),
      maxVoteWeight: new BN(100),
      voteThreshold: new BN(1),
      name: "TestProposal",
      vetoVoteWeight: new BN(10),
      elGamalPublicKey: voteAdminElGamalSecretKey.publicKey,
    };

    const feeUtxo = new Utxo({
      poseidon: POSEIDON,
      assets: [SystemProgram.programId],
      publicKey: voter.pubkey,
      amounts: [new BN(1e9)],
      verifierAddress: verifierProgramId,
      assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
      index: 0,
    });

    const voteWeightUtxoData: VoteWeightUtxoData = {
      voteWeight: new BN(1e9),
      releaseSlot: new BN(1000),
      rate: new BN(0),
      voteLock: new BN(0),
      voteUtxoId: new BN(123),
      // TODO: once we have a separate vote weight psp program, we can use that here.
      voteWeightPspAddress: hashAndTruncateToCircuit(
        verifierProgramId.toBytes()
      ),
    };

    const voteWeightUtxo = new Utxo({
      poseidon: POSEIDON,
      assets: [SystemProgram.programId],
      publicKey: voter.pubkey,
      amounts: [new BN(1e9)],
      appData: voteWeightUtxoData,
      appDataIdl: IDL,
      verifierAddress: verifierProgramId,
      assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
      index: 1,
    });

    const merkleTree = new MerkleTree(18, POSEIDON, [
      feeUtxo.getCommitment(POSEIDON),
      voteWeightUtxo.getCommitment(POSEIDON),
    ]);
    const solMerkleTree = new SolMerkleTree({
      pubkey: MerkleTreeConfig.getTransactionMerkleTreePda(new BN(0)),
      merkleTree,
      poseidon: POSEIDON,
    });
    const currentSlot = new BN(10);
    const circuitPath = path.join("build-circuit/private-voting/privateVoting");

    const noZeroNonce = generateRandomSalt();
    const { ephemeralKey: zeroNoEmphemeralKey, ciphertext: zeroNoCiphertext } =
      encrypt(voteParameters.elGamalPublicKey, BigInt(0), noZeroNonce);
    const zeroCiphertextArray = pointToStringArray(zeroNoCiphertext).map((x) =>
      new BN(x).toArray("be", 32)
    );
    const zeroEmphemeralArray = pointToStringArray(zeroNoEmphemeralKey).map(
      (x) => new BN(x).toArray("be", 32)
    );

    const voteTransactionInput: VoteTransactionInput = {
      voteWeightUtxo,
      feeUtxo,
      voteParameters,
      idl: IDL,
      lookUpTables: lightProvider.lookUpTables,
      proofInputs: {
        currentSlot,
        publicElGamalPublicKeyX: new BN(
          voteParameters.elGamalPublicKey.ex.toString()
        ),
        publicElGamalPublicKeyY: new BN(
          voteParameters.elGamalPublicKey.ey.toString()
        ),
        publicOldVoteWeightNoEmphemeralKeyX: new BN(zeroEmphemeralArray[0]),
        publicOldVoteWeightNoEmphemeralKeyY: new BN(zeroEmphemeralArray[1]),
        publicOldVoteWeightYesEmphemeralKeyX: new BN(zeroEmphemeralArray[0]),
        publicOldVoteWeightYesEmphemeralKeyY: new BN(zeroEmphemeralArray[1]),
        publicOldVoteWeightNoCiphertextX: new BN(zeroCiphertextArray[0]),
        publicOldVoteWeightNoCiphertextY: new BN(zeroCiphertextArray[1]),
        publicOldVoteWeightYesCiphertextX: new BN(zeroCiphertextArray[0]),
        publicOldVoteWeightYesCiphertextY: new BN(zeroCiphertextArray[1]),
      },
      voter,
      circuitPath,
      relayer: localTestRelayer,
      solMerkleTree,
      voteYes: true,
    };
    await createAndProveVoteTransaction(voteTransactionInput);
  });
});

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
  releaseSlot: BN;
  rate: BN;
  voteLock: BN;
  voteUtxoId: BN;
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
  // publicOldVoteWeightYesEmphemeralKey: ExtPointType;
  // publicOldVoteWeightYes: ExtPointType;
  // publicOldVoteWeightNoEmphemeralKey: ExtPointType;
  // publicOldVoteWeightNo: ExtPointType;
};

export const createAndProveVoteTransaction = async (
  voteTransactionInput: VoteTransactionInput
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
    poseidon: POSEIDON,
    assets: voteWeightUtxo.assets,
    publicKey: voteWeightUtxo.publicKey,
    amounts: voteWeightUtxo.amounts,
    appData: lockedVoteWeightUtxoData,
    appDataIdl: idl,
    verifierAddress: voteWeightUtxo.verifierAddress,
    assetLookupTable: lookUpTables.assetLookupTable,
  });

  const changeUtxo = new Utxo({
    poseidon: POSEIDON,
    assets: [SystemProgram.programId],
    publicKey: voteWeightUtxo.publicKey,
    amounts: [feeUtxo.amounts[0].sub(relayer.relayerFee)],
    verifierAddress: verifierProgramId,
    assetLookupTable: lookUpTables.assetLookupTable,
  });

  const publicVoteId = new BN(utils.bytes.utf8.encode("publicVoteId"));

  const nullifier = new BN(
    POSEIDON.F.toString(
      POSEIDON([voteWeightUtxo.appData.voteUtxoId, publicVoteId])
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
      // publicElGamalPublicKeyX: proofInputs.publicElGamalPublicKeyX,
      // publicElGamalPublicKeyY: proofInputs.publicElGamalPublicKeyY,
      voteWeightNullifier: nullifier,
      publicVoteEnd: voteParameters.votingCompletedAt,
      // currentSlot: proofInputs.currentSlot,
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
    POSEIDON,
    voter,
    relayer
  );

  const internalProofInputs = createProofInputs({
    poseidon: POSEIDON,
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

export const createPspTransaction = async (
  pspTransactionInput: PspTransactionInput,
  poseidon: any,
  account: Account,
  relayer: Relayer
): Promise<TransactionParameters> => {
  let inputUtxos: Utxo[] = [];
  if (pspTransactionInput.checkedInUtxos) {
    inputUtxos = [
      ...pspTransactionInput.checkedInUtxos.map((item) => item.utxo),
    ];
  }
  if (pspTransactionInput.inUtxos) {
    inputUtxos = [...inputUtxos, ...pspTransactionInput.inUtxos];
  }
  let outputUtxos: Utxo[] = [];
  if (pspTransactionInput.checkedOutUtxos) {
    outputUtxos = [
      ...pspTransactionInput.checkedOutUtxos.map((item) => item.utxo),
    ];
  }
  if (pspTransactionInput.outUtxos) {
    outputUtxos = [...outputUtxos, ...pspTransactionInput.outUtxos];
  }

  const txParams = new TransactionParameters({
    inputUtxos,
    outputUtxos,
    transactionMerkleTreePubkey: MerkleTreeConfig.getTransactionMerkleTreePda(
      new BN(0)
    ),
    eventMerkleTreePubkey: MerkleTreeConfig.getEventMerkleTreePda(new BN(0)),
    action: Action.TRANSFER,
    poseidon: poseidon,
    relayer: relayer,
    verifierIdl: IDL_LIGHT_PSP4IN4OUT_APP_STORAGE,
    account: account,
    verifierState: getVerifierStatePda(
      verifierProgramId,
      relayer.accounts.relayerPubkey
    ),
  });

  await txParams.getTxIntegrityHash(POSEIDON);
  return txParams;
};
