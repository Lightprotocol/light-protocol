import * as anchor from "@coral-xyz/anchor";
import { assert } from "chai";
import {
  TestRelayer,
  Provider,
  circuitlibjs,
  Account,
} from "@lightprotocol/zk.js";
import { Prover } from "@lightprotocol/prover.js";
const { MerkleTree, ElGamalUtils } = circuitlibjs;
const { pointToStringArray, coordinatesToExtPoint } = ElGamalUtils;
import { PublicKey, Keypair } from "@solana/web3.js";
import {
  encrypt,
  PublicKey as ElGamalPublicKey,
  generateKeypair,
  generateRandomSalt,
} from "@lightprotocol/circuit-lib.js";

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
  let lightProvider: Provider, localTestRelayer: TestRelayer;

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
  });
  it("test serialization ", async () => {
    const proposerElGamalKeypair = generateKeypair();

    const yesZeroNonce = generateRandomSalt();
    const {
      ephemeralKey: zeroYesEmphemeralKey,
      ciphertext: zeroYesCiphertext,
    } = encrypt(proposerElGamalKeypair.publicKey, BigInt(0), yesZeroNonce);
    console.log("zeroYesCiphertext ", zeroYesCiphertext);

    const zeroCiphertextString = pointToStringArray(zeroYesCiphertext);
    const zeroEmphemeralKeyString = pointToStringArray(zeroYesEmphemeralKey);
    const elGamalPublicKeyString = pointToStringArray(
      proposerElGamalKeypair.publicKey
    );

    const zeroCiphertextBN = new BN(zeroCiphertextString[0]);
    const zeroCiphertextBN2 = new BN(zeroCiphertextString[1]);

    const zeroCiphertextExt = coordinatesToExtPoint<BigInt>(
      BigInt(zeroCiphertextBN.toString()),
      BigInt(zeroCiphertextBN2.toString())
    );
    console.log("zeroCiphertextExt ", zeroCiphertextExt);

    const zeroCiphertextArray = new BN(zeroCiphertextString[0]).toArray(
      "be",
      32
    );
    const zeroCiphertextArray2 = new BN(zeroCiphertextString[1]).toArray(
      "be",
      32
    );
    const zeroCiphertextExt2 = coordinatesToExtPoint<BigInt>(
      BigInt(new BN(zeroCiphertextArray).toString()),
      BigInt(new BN(zeroCiphertextArray2).toString())
    );
    console.log("zeroCiphertextExt2 ", zeroCiphertextExt2);
  });

  it(" test init vote circuit ", async () => {
    const voteAdminElGamalSecretKey = generateKeypair();

    const circuitPath = path.join("build-circuit");

    const initVoteTransactionInput: InitVoteTransactionInput = {
      idl: IDL,
      elGamalPublicKey: voteAdminElGamalSecretKey.publicKey,
      circuitPath,
    };
    await createInitVoteProof(initVoteTransactionInput);
  });
});

export type InitVoteTransactionInput = {
  idl: anchor.Idl;
  elGamalPublicKey: ElGamalPublicKey;
  circuitPath: string;
};

export const createInitVoteProof = async (
  voteTransactionInput: InitVoteTransactionInput
) => {
  const { idl, circuitPath, elGamalPublicKey } = voteTransactionInput;
  const yesZeroNonce = generateRandomSalt();
  const { ephemeralKey: zeroYesEmphemeralKey, ciphertext: zeroYesCiphertext } =
    encrypt(elGamalPublicKey, BigInt(0), yesZeroNonce);

  const zeroCiphertextString = pointToStringArray(zeroYesCiphertext);
  const zeroEmphemeralKeyString = pointToStringArray(zeroYesEmphemeralKey);
  const elGamalPublicKeyString = pointToStringArray(elGamalPublicKey);

  const publicInputs = {
    publicElGamalPublicKeyX: new BN(elGamalPublicKeyString[0]),
    publicElGamalPublicKeyY: new BN(elGamalPublicKeyString[1]),
    publicZeroYesEmphemeralKeyX: new BN(zeroEmphemeralKeyString[0]),
    publicZeroYesEmphemeralKeyY: new BN(zeroEmphemeralKeyString[1]),
    publicZeroYesCiphertextX: new BN(zeroCiphertextString[0]),
    publicZeroYesCiphertextY: new BN(zeroCiphertextString[1]),
  };
  const proofInputs = {
    ...publicInputs,
    nonce: new BN(yesZeroNonce.toString()),
  };
  const prover = new Prover(idl, circuitPath, "initVote");
  await prover.addProofInputs(proofInputs);
  console.time("Init vote proof: ");
  const { parsedProof, parsedPublicInputs } = await prover.fullProveAndParse();
  console.timeEnd("Init vote proof: ");
  return { proof: parsedProof, publicInputs: parsedPublicInputs };
};
