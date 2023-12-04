import * as anchor from "@coral-xyz/anchor";
import { TestRelayer, Provider, circuitlibjs } from "@lightprotocol/zk.js";
const { ElGamalUtils } = circuitlibjs;
const { pointToStringArray } = ElGamalUtils;
import { PublicKey, Keypair } from "@solana/web3.js";
import {
  encrypt,
  generateKeypair,
  generateRandomSalt,
  formatSecretKey,
} from "@lightprotocol/circuit-lib.js";

import { buildPoseidonOpt } from "circomlibjs";
import { BN } from "@coral-xyz/anchor";
import { IDL, PrivateVoting } from "../target/types/private_voting";
import { utils } from "@project-serum/anchor";
import {
  createPublishDecryptedTallyProof,
  PublishDecryptedTallyTransactionInput,
} from "../sdk";
const path = require("path");

const verifierProgramId = new PublicKey(
  "Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS"
);

const RPC_URL = "http://127.0.0.1:8899";

/**
 * 1. create proposal
 * 2. create vote utxo
 * 3. init vote
 * 4. vote
 */
describe("Test Decrypt Tally Circuit", () => {
  process.env.ANCHOR_PROVIDER_URL = RPC_URL;
  process.env.ANCHOR_WALLET = process.env.HOME + "/.config/solana/id.json";

  // Configure the client to use the local cluster.
  let proposerKeypair: Keypair,
    voterKeypair: Keypair,
    voteProgram: anchor.Program<PrivateVoting> = anchor.workspace.PrivateVoting;
  let proposalPda: PublicKey;
  let lightProvider: Provider, localTestRelayer: TestRelayer;
  let POSEIDON: any;
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

  it(" test init vote circuit ", async () => {
    const voteAdminElGamalSecretKey = generateKeypair();

    const yesNonce = formatSecretKey(generateRandomSalt());
    let { ephemeralKey: yesEmphemeralKey, ciphertext: yesCiphertext } = encrypt(
      voteAdminElGamalSecretKey.publicKey,
      BigInt(65536),
      yesNonce
    );
    const noNonce = formatSecretKey(generateRandomSalt());

    let { ephemeralKey: noEmphemeralKey, ciphertext: noCiphertext } = encrypt(
      voteAdminElGamalSecretKey.publicKey,
      BigInt(1e8),
      noNonce
    );

    // Test homomorphic addition
    // yesEmphemeralKey = yesEmphemeralKey.add(noEmphemeralKey);
    // yesCiphertext = yesCiphertext.add(noCiphertext);
    // noEmphemeralKey = noEmphemeralKey.add(yesEmphemeralKey);
    // noCiphertext = noCiphertext.add(yesCiphertext);

    const yesCiphertextString = pointToStringArray(yesCiphertext);
    const yesEmphemeralKeyString = pointToStringArray(yesEmphemeralKey);

    const noCiphertextString = pointToStringArray(noCiphertext);
    const noEmphemeralKeyString = pointToStringArray(noEmphemeralKey);

    const circuitPath = path.join(
      "build-circuit/private-voting/publishDecryptedTally"
    );

    const initVoteTransactionInput: PublishDecryptedTallyTransactionInput = {
      idl: IDL,
      proofInputs: {
        publicVoteWeightNoEmphemeralKeyX: new BN(noEmphemeralKeyString[0]),
        publicVoteWeightNoEmphemeralKeyY: new BN(noEmphemeralKeyString[1]),
        publicVoteWeightYesEmphemeralKeyX: new BN(yesEmphemeralKeyString[0]),
        publicVoteWeightYesEmphemeralKeyY: new BN(yesEmphemeralKeyString[1]),
        publicVoteWeightNoX: new BN(noCiphertextString[0]),
        publicVoteWeightNoY: new BN(noCiphertextString[1]),
        publicVoteWeightYesX: new BN(yesCiphertextString[0]),
        publicVoteWeightYesY: new BN(yesCiphertextString[1]),
      },
      secretKey: voteAdminElGamalSecretKey.secretKey,
      circuitPath,
    };
    await createPublishDecryptedTallyProof(initVoteTransactionInput);
  });
});
