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
  setUndefinedPspCircuitInputsToZero,
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

import { createPspTransaction, VoteWeightUtxoData } from "./vote-circuit.test";

import { buildPoseidonOpt } from "circomlibjs";
import { BN } from "@coral-xyz/anchor";
import { IDL, VoteWeightProgram } from "../target/types/vote_weight_program";
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
describe("Test create vote weight utxo", () => {
  process.env.ANCHOR_PROVIDER_URL = RPC_URL;
  process.env.ANCHOR_WALLET = process.env.HOME + "/.config/solana/id.json";

  // Configure the client to use the local cluster.
  let proposerKeypair: Keypair,
    voterKeypair: Keypair,
    voteProgram: anchor.Program<VoteWeightProgram> =
      anchor.workspace.PrivateVoting;
  let proposalPda: PublicKey;
  let voter: Account, lightProvider: Provider, localTestRelayer: TestRelayer;

  before(async () => {
    POSEIDON = await buildPoseidonOpt();
    proposerKeypair = Keypair.generate();
    voterKeypair = Keypair.generate();

    voteProgram = new anchor.Program<VoteWeightProgram>(IDL, verifierProgramId);
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

  it.only("test circuit: create vote weight utxo ", async () => {
    const circuitPath = path.join(
      "build-circuit/vote-weight-program/createVoteUtxo"
    );

    const feeUtxo = new Utxo({
      poseidon: POSEIDON,
      assets: [SystemProgram.programId],
      publicKey: voter.pubkey,
      amounts: [new BN(1e9)],
      verifierAddress: verifierProgramId,
      assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
      index: 0,
    });
    const inUtxo = new Utxo({
      poseidon: POSEIDON,
      assets: [SystemProgram.programId],
      publicKey: voter.pubkey,
      amounts: [new BN(1e9)],
      verifierAddress: verifierProgramId,
      assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
      index: 1,
    });

    const merkleTree = new MerkleTree(18, POSEIDON, [
      feeUtxo.getCommitment(POSEIDON),
      inUtxo.getCommitment(POSEIDON),
    ]);
    const solMerkleTree = new SolMerkleTree({
      pubkey: MerkleTreeConfig.getTransactionMerkleTreePda(new BN(0)),
      merkleTree,
      poseidon: POSEIDON,
    });
    const createVoteWeightUtxoTransactionInput: createVoteWeightUtxoTransactionInput =
      {
        inUtxos: [inUtxo],
        feeUtxo,
        idl: IDL,
        lookUpTables: lightProvider.lookUpTables,
        voter,
        circuitPath,
        relayer: localTestRelayer,
        solMerkleTree,
        startSlot: BN_0,
        voteUtxoNumber: BN_0,
        timeLocked: new BN(10),
        voteWeightCreationParamatersPda: {
          governingTokenMint: BN_0,
          voteUtxoNumber: BN_0,
          publicMaxLockTime: new BN(100),
        },
        publicCurrentSlot: new BN(0),
        voteWeightAmount: new BN(1e9),
        voteWeightConfig: SystemProgram.programId, // just a dummy value
        verifierProgramId, // just a dummy value
        voteWeightProgramId: SystemProgram.programId, // just a dummy value
      };
    await createAndProveCreateVoteUtxoTransaction(
      createVoteWeightUtxoTransactionInput,
      POSEIDON
    );
  });

  it("test circuit: claim vote weight utxo ", async () => {
    const circuitPath = path.join(
      "build-circuit/vote-weight-program/createVoteUtxo"
    );
    const publicCurrentSlot = new BN(100);
    const timeLocked = new BN(10);
    const voteWeightAmount = new BN(1e9);
    const voteUtxoNumber = new BN(0);

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
        SystemProgram.programId.toBytes()
      ),
    };
    const voteWeightUtxo = new Utxo({
      poseidon: POSEIDON,
      assets: [SystemProgram.programId],
      publicKey: voter.pubkey,
      amounts: [voteWeightAmount],
      assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
      appData: voteWeightUtxoData,
      appDataIdl: IDL,
      verifierAddress: verifierProgramId,
      index: 1,
    });
    const feeUtxo = new Utxo({
      poseidon: POSEIDON,
      assets: [SystemProgram.programId],
      publicKey: voter.pubkey,
      amounts: [new BN(1e9)],
      verifierAddress: verifierProgramId,
      assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
      index: 0,
    });
    console.log(
      `\n\n ----------------  Creating vote weight utxo: vote weight ${voteWeightUtxoData.voteWeight} ---------------- \n\n`
    );

    const merkleTree = new MerkleTree(18, POSEIDON, [
      feeUtxo.getCommitment(POSEIDON),
      voteWeightUtxo.getCommitment(POSEIDON),
    ]);
    const solMerkleTree = new SolMerkleTree({
      pubkey: MerkleTreeConfig.getTransactionMerkleTreePda(new BN(0)),
      merkleTree,
      poseidon: POSEIDON,
    });
    const createVoteWeightUtxoTransactionInput: claimVoteWeightUtxoTransactionInput =
      {
        voteWeightUtxo,
        feeUtxo,
        idl: IDL,
        lookUpTables: lightProvider.lookUpTables,
        voter,
        circuitPath,
        relayer: localTestRelayer,
        solMerkleTree,
        voteWeightCreationParamatersPda: {
          governingTokenMint: BN_0,
          voteUtxoNumber: BN_0,
          publicMaxLockTime: new BN(100),
        },
        publicCurrentSlot,
        voteWeightConfig: SystemProgram.programId, // just a dummy value
        verifierProgramId, // just a dummy value
        voteWeightProgramId: SystemProgram.programId, // just a dummy value
      };
    await createAndProveClaimVoteUtxoTransaction(
      createVoteWeightUtxoTransactionInput,
      POSEIDON
    );
  });
});
export type voteWeightCreationParamatersPda = {
  governingTokenMint: BN;
  voteUtxoNumber: BN;
  publicMaxLockTime: BN;
};
export type createVoteWeightUtxoTransactionInput = {
  inUtxos: Utxo[];
  feeUtxo?: Utxo;
  voteWeightCreationParamatersPda: voteWeightCreationParamatersPda;
  idl: anchor.Idl;
  lookUpTables: { assetLookupTable: any; verifierProgramLookupTable: any };
  voter: Account;
  circuitPath: string;
  relayer: Relayer;
  solMerkleTree: SolMerkleTree;
  startSlot: BN;
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
    lookUpTables,
    voter,
    circuitPath,
    relayer,
    solMerkleTree,
    startSlot,
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
    assets: [SystemProgram.programId],
    publicKey: voter.pubkey,
    amounts: [voteWeightAmount],
    appData: voteWeightUtxoData,
    appDataIdl: idl,
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

  const pspTransactionInput: PspTransactionInput = {
    proofInputs: {},
    path: circuitPath,
    verifierIdl: idl,
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
    ...voteWeightCreationParamatersPda,
    publicCurrentSlot,
    publicVoteUtxoNumber: voteWeightCreationParamatersPda.voteUtxoNumber,
    publicGoverningTokenMint:
      voteWeightCreationParamatersPda.governingTokenMint,
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
    idl,
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
  console.log("pspProof ", pspProof.parsedPublicInputsObject);
  return { systemProof, pspProof, transaction, pspTransactionInput };
};

export type claimVoteWeightUtxoTransactionInput = {
  voteWeightUtxo: Utxo;
  feeUtxo?: Utxo;
  voteWeightCreationParamatersPda: voteWeightCreationParamatersPda;
  idl: anchor.Idl;
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
    verifierIdl: idl,
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
    ...voteWeightCreationParamatersPda,
    publicCurrentSlot,
    publicVoteUtxoNumber: voteWeightCreationParamatersPda.voteUtxoNumber,
    publicGoverningTokenMint:
      voteWeightCreationParamatersPda.governingTokenMint,
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
    idl,
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
  return { systemProof, pspProof, transaction, pspTransactionInput };
};
