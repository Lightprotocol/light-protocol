import * as anchor from "@coral-xyz/anchor";
import {
  Utxo,
  TestRelayer,
  Provider,
  hashAndTruncateToCircuit,
  MerkleTreeConfig,
  circuitlibjs,
  Account,
  SolMerkleTree,
} from "@lightprotocol/zk.js";
const { MerkleTree, ElGamalUtils } = circuitlibjs;
const { pointToStringArray } = ElGamalUtils;
import { SystemProgram, PublicKey, Keypair } from "@solana/web3.js";
import {
  encrypt,
  generateKeypair,
  generateRandomSalt,
} from "@lightprotocol/circuit-lib.js";

import {
  VoteParameters,
  VoteTransactionInput,
  VoteWeightUtxoData,
  createAndProveVoteTransaction,
} from "../sdk/index";
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
      startSlot: new BN(0),
      releaseSlot: new BN(10),
      rate: new BN(0),
      voteLock: new BN(0),
      voteUtxoNumber: new BN(0),
      voteUtxoIdNonce: new BN(0),
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
    await createAndProveVoteTransaction(voteTransactionInput, POSEIDON);
  });
});
