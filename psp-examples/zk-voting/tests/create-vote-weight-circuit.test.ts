import {
  Utxo,
  TestRelayer,
  Provider,
  hashAndTruncateToCircuit,
  MerkleTreeConfig,
  circuitlibjs,
  Account,
  SolMerkleTree,
  BN_0,
} from "@lightprotocol/zk.js";
const { MerkleTree } = circuitlibjs;

import { SystemProgram, PublicKey, Keypair } from "@solana/web3.js";

import {
  claimVoteWeightUtxoTransactionInput,
  createAndProveClaimVoteUtxoTransaction,
  createAndProveCreateVoteUtxoTransaction,
  createPspTransaction,
  createVoteWeightUtxoTransactionInput,
  VoteWeightUtxoData,
} from "../sdk/index";

import { buildPoseidonOpt } from "circomlibjs";
import { BN } from "@coral-xyz/anchor";
import { IDL } from "../target/types/vote_weight_program";
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
describe("Test Create Vote Weight Utxo Circuit", () => {
  process.env.ANCHOR_PROVIDER_URL = RPC_URL;
  process.env.ANCHOR_WALLET = process.env.HOME + "/.config/solana/id.json";

  // Configure the client to use the local cluster.
  let proposerKeypair: Keypair, voterKeypair: Keypair;
  let voter: Account, lightProvider: Provider, localTestRelayer: TestRelayer;

  before(async () => {
    POSEIDON = await buildPoseidonOpt();
    proposerKeypair = Keypair.generate();
    voterKeypair = Keypair.generate();

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

  it("test circuit: create vote weight utxo ", async () => {
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
        pspIdl: IDL,
        lookUpTables: lightProvider.lookUpTables,
        voter,
        circuitPath,
        relayer: localTestRelayer,
        solMerkleTree,
        voteUtxoNumber: BN_0,
        timeLocked: new BN(10),
        voteWeightCreationParamatersPda: {
          governingTokenMint: SystemProgram.programId,
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
      releaseSlot: timeLocked,
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
        pspIdl: IDL,
        lookUpTables: lightProvider.lookUpTables,
        voter,
        circuitPath,
        relayer: localTestRelayer,
        solMerkleTree,
        voteWeightCreationParamatersPda: {
          governingTokenMint: SystemProgram.programId,
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
