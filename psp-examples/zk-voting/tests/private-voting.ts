import * as anchor from "@coral-xyz/anchor";
import { assert } from "chai";
import {
  createAndProveVoteTransaction,
  VoteParameters,
  VoteTransactionInput,
  VoteWeightUtxoData,
} from "./vote-circuit.test";
import {
  Utxo,
  confirmConfig,
  TestRelayer,
  User,
  airdropSol,
  Provider,
  hashAndTruncateToCircuit,
  SolanaTransactionInputs,
  circuitlibjs,
  Action,
  ProgramUtxoBalance,
  ConfirmOptions,
  sendAndConfirmShieldedTransaction,
} from "@lightprotocol/zk.js";

import {
  SystemProgram,
  PublicKey,
  Keypair,
  sendAndConfirmTransaction,
  ComputeBudgetProgram,
} from "@solana/web3.js";

import { buildPoseidonOpt } from "circomlibjs";
import { BN, utils } from "@coral-xyz/anchor";
import { IDL, PrivateVoting } from "../target/types/private_voting";
import { createInitVoteProof } from "./init-vote-circuit.test";
import {
  createPublishDecryptedTallyProof,
  PublishDecryptedTallyTransactionInput,
} from "./publish-decrypted-tally-circuit.test";
import {
  decode,
  decrypt,
  ElGamalUtils,
  encrypt,
  formatSecretKey,
  generateKeypair,
  generateRandomSalt,
} from "@lightprotocol/circuit-lib.js";
const path = require("path");
const { coordinatesToExtPoint, pointToStringArray } = ElGamalUtils;

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
 * 5. decrypt and publish results
 */
describe("Test private-voting", () => {
  process.env.ANCHOR_PROVIDER_URL = RPC_URL;
  process.env.ANCHOR_WALLET = process.env.HOME + "/.config/solana/id.json";

  // Configure the client to use the local cluster.
  const provider = anchor.AnchorProvider.local(RPC_URL, confirmConfig);
  anchor.setProvider(provider);
  let proposerKeypair: Keypair,
    voterKeypair: Keypair,
    voteProgram: anchor.Program<PrivateVoting> = anchor.workspace.PrivateVoting;
  let voteParameters: VoteParameters,
    proposalPda: PublicKey,
    votePda: PublicKey;
  let voteWeightUtxo: Utxo,
    voter: User,
    lightProvider: Provider,
    localTestRelayer: TestRelayer,
    proposerElGamalKeypair: circuitlibjs.Keypair;

  before(async () => {
    POSEIDON = await buildPoseidonOpt();
    proposerKeypair = Keypair.generate();
    proposerElGamalKeypair = generateKeypair();

    voterKeypair = Keypair.generate();
    await airdropSol({
      connection: provider.connection,
      lamports: 1e10,
      recipientPublicKey: proposerKeypair.publicKey,
    });
    await airdropSol({
      connection: provider.connection,
      lamports: 1e10,
      recipientPublicKey: voterKeypair.publicKey,
    });

    voteProgram = new anchor.Program<PrivateVoting>(
      IDL,
      verifierProgramId,
      provider
    );
    proposalPda = PublicKey.findProgramAddressSync(
      [
        proposerKeypair.publicKey.toBuffer(),
        utils.bytes.utf8.encode("MockProposalV2"),
      ],
      verifierProgramId
    )[0];

    const relayerWallet = Keypair.generate();
    await airdropSol({
      connection: provider.connection,
      lamports: 1e10,
      recipientPublicKey: relayerWallet.publicKey,
    });

    localTestRelayer = new TestRelayer({
      relayerPubkey: relayerWallet.publicKey,
      relayerRecipientSol: relayerWallet.publicKey,
      relayerFee: new BN(100000),
      payer: relayerWallet,
    });

    lightProvider = await Provider.init({
      wallet: voterKeypair,
      url: RPC_URL,
      relayer: localTestRelayer,
      confirmConfig,
    });
    voter = await User.init({ provider: lightProvider });
  });

  it("create proposal: ", async () => {
    console.log(`\n\n ----------------  Creating mock proposal ---------------- \n\n`)

    voteParameters = {
      governingTokenMint: SystemProgram.programId,
      startVotingAt: new BN(0),
      votingCompletedAt: new BN(1000),
      maxVoteWeight: new BN(100),
      voteThreshold: new BN(1),
      name: "TestProposal",
      vetoVoteWeight: new BN(10),
      elGamalPublicKey: proposerElGamalKeypair.publicKey,
    };

    const initProposalInstruction = await voteProgram.methods
      .initMockProposal(
        voteParameters.governingTokenMint,
        voteParameters.startVotingAt,
        voteParameters.votingCompletedAt,
        voteParameters.maxVoteWeight,
        voteParameters.voteThreshold,
        voteParameters.name,
        voteParameters.vetoVoteWeight
      )
      .accounts({
        signer: proposerKeypair.publicKey,
        proposal: proposalPda,
      })
      .instruction();
    const initProposalTxHash = await sendAndConfirmTransaction(
      provider.connection,
      new anchor.web3.Transaction().add(initProposalInstruction),
      [proposerKeypair],
      {
        commitment: "confirmed",
      }
    );
    console.log("Init Proposal Tx Hash: ", initProposalTxHash);
    const proposalAccountInfo = await voteProgram.account.mockProposalV2.fetch(
      proposalPda
    );
    assert.equal(proposalAccountInfo.name, voteParameters.name);
    assert.equal(
      proposalAccountInfo.governingTokenMint.toBase58(),
      voteParameters.governingTokenMint.toBase58()
    );
    assert.equal(
      proposalAccountInfo.startVotingAt.toString(),
      voteParameters.startVotingAt.toString()
    );
    assert.equal(
      proposalAccountInfo.votingCompletedAt.toString(),
      voteParameters.votingCompletedAt.toString()
    );
    assert.equal(
      proposalAccountInfo.maxVoteWeight.toString(),
      voteParameters.maxVoteWeight.toString()
    );
    assert.equal(
      proposalAccountInfo.voteThreshold.toString(),
      voteParameters.voteThreshold.toString()
    );
    assert.equal(
      proposalAccountInfo.vetoVoteWeight.toString(),
      voteParameters.vetoVoteWeight.toString()
    );
  });

  it("create vote weight utxo: ", async () => {
    const voteWeightUtxoData: VoteWeightUtxoData = {
      voteWeight: new BN(1e9),
      releaseSlot: new BN(0),
      rate: new BN(0),
      voteLock: new BN(0),
      voteUtxoId: new BN(0),
      // TODO: once we have a separate vote weight psp program, we can use that here.
      voteWeightPspAddress: hashAndTruncateToCircuit(
        verifierProgramId.toBytes()
      ),
    };
    console.log(`\n\n ----------------  Creating vote weight utxo: vote weight ${voteWeightUtxoData.voteWeight} ---------------- \n\n`)

    voteWeightUtxo = new Utxo({
      poseidon: POSEIDON,
      assets: [SystemProgram.programId],
      publicKey: voter.account.pubkey,
      amounts: [new BN(1e9)],
      appData: voteWeightUtxoData,
      appDataIdl: IDL,
      verifierAddress: verifierProgramId,
      assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
    });
    let storeTransaction = await voter.storeAppUtxo({
      appUtxo: voteWeightUtxo,
      action: Action.SHIELD,
    });

    console.log(
      "store program utxo transaction hash ",
      storeTransaction.txHash
    );

    const programUtxoBalance: Map<string, ProgramUtxoBalance> =
      await voter.syncStorage(IDL);
    const shieldedUtxoCommitmentHash = voteWeightUtxo.getCommitment(POSEIDON);
    const inputUtxo = programUtxoBalance
      .get(verifierProgramId.toBase58())
      .tokenBalances.get(voteWeightUtxo.assets[1].toBase58())
      .utxos.get(shieldedUtxoCommitmentHash);
    Utxo.equal(POSEIDON, inputUtxo, voteWeightUtxo, true);
  });

  it("init vote: ", async () => {

    votePda = PublicKey.findProgramAddressSync(
      [proposalPda.toBuffer(), utils.bytes.utf8.encode("VOTE")],
      verifierProgramId
    )[0];
    console.log(`\n\n ----------------  Creating vote pda ${votePda.toBase58()} ---------------- \n\n`)

    const initVoteTransactionInput = {
      idl: IDL,
      elGamalPublicKey: proposerElGamalKeypair.publicKey,
      circuitPath: path.join("build-circuit/private-voting/initVote"),
    };
    const { proof, publicInputs } = await createInitVoteProof(
      initVoteTransactionInput
    );
    const initVoteInstruction = await voteProgram.methods
      .initVote(
        publicInputs[0],
        publicInputs[1],
        publicInputs[2],
        publicInputs[3],
        publicInputs[4],
        publicInputs[5],
        proof.proofA,
        proof.proofB.flat(),
        proof.proofC
      )
      .accounts({
        signer: proposerKeypair.publicKey,
        proposal: proposalPda,
        votePda,
        systemProgram: SystemProgram.programId,
      })
      .signers([proposerKeypair])
      .instruction();
    try {
      const initVoteTxHash = await sendAndConfirmTransaction(
        provider.connection,
        new anchor.web3.Transaction().add(initVoteInstruction),
        [proposerKeypair],
        {
          commitment: "confirmed",
        }
      );
      console.log("Init Vote Tx Hash: ", initVoteTxHash);
    } catch (e) {
      console.log("error ", e);
    }

    const voteAccountInfo = await voteProgram.account.votePda.fetch(votePda);
    const proposalAccountInfo = await voteProgram.account.mockProposalV2.fetch(
      proposalPda
    );
    assert.equal(
      voteAccountInfo.proposalPda.toBase58(),
      proposalPda.toBase58()
    );
    assert.equal(
      voteAccountInfo.governingTokenMint.toBase58(),
      proposalAccountInfo.governingTokenMint.toBase58()
    );
    assert.equal(
      voteAccountInfo.slotVoteStart.toString(),
      proposalAccountInfo.startVotingAt.toString()
    );
    assert.equal(
      voteAccountInfo.slotVoteEnd.toString(),
      proposalAccountInfo.votingCompletedAt.toString()
    );
    assert.equal(
      voteAccountInfo.maxVoteWeight.toString(),
      proposalAccountInfo.maxVoteWeight.toString()
    );
    assert.equal(
      voteAccountInfo.encryptedYesVotes.toString(),
      [
        publicInputs[2],
        publicInputs[3],
        publicInputs[4],
        publicInputs[5],
      ].toString()
    );
    assert.equal(
      voteAccountInfo.encryptedNoVotes.toString(),
      [
        publicInputs[2],
        publicInputs[3],
        publicInputs[4],
        publicInputs[5],
      ].toString()
    );
    assert.equal(
      voteAccountInfo.decryptedYesVoteWeight.toString(),
      new BN(0).toString()
    );
    assert.equal(
      voteAccountInfo.decryptedNoVoteWeight.toString(),
      new BN(0).toString()
    );
    const extPointNoCiphertext = coordinatesToExtPoint<BigInt>(
      BigInt(new BN(voteAccountInfo.encryptedYesVotes[2], 32, "be").toString()),
      BigInt(new BN(voteAccountInfo.encryptedYesVotes[3], 32, "be").toString())
    );
    const extPointNoEmphemeralKey = coordinatesToExtPoint<BigInt>(
      BigInt(new BN(voteAccountInfo.encryptedNoVotes[0]).toString()),
      BigInt(new BN(voteAccountInfo.encryptedNoVotes[1]).toString())
    );

    const decryptedNo = decrypt(
      proposerElGamalKeypair.secretKey,
      extPointNoEmphemeralKey,
      extPointNoCiphertext
    );
    let directoryPath = "../../circuit-lib/circuit-lib.js/build";
    const fs = require("fs");
    const lookupTable19Path = directoryPath + `/lookupTableBBJub19.json`;
    const lookupTable = JSON.parse(fs.readFileSync(lookupTable19Path));

    const decodedNo = decode(decryptedNo, 19, lookupTable);
    assert.equal(decodedNo.value.toString(), "0");
    // console.log(`\n\n Vote account info \n\n`);
    // console.log("max vote weight ", voteAccountInfo.maxVoteWeight.toString());
    // console.log("encrypted yes votes ", voteAccountInfo.encryptedYesVotes.toString());
    // console.log("encrypted no votes ", voteAccountInfo.encryptedNoVotes.toString());
    // console.log("decrypted yes vote weight ", voteAccountInfo.decryptedYesVoteWeight.toString());
    // console.log("decrypted no vote weight ", voteAccountInfo.decryptedNoVoteWeight.toString());
  });

  it("test vote ", async () => {
    /**
     * in utxos
     * 1. vote weight utxo, 2. fee utxo
     * out utxos
     * 1. locked vote weight utxo, 2. change fee utxo
     */
    console.log(`\n\n ----------------  Shielding 1 sol ---------------- \n\n`)

    // create fee utxo
    await voter.shield({ token: "SOL", publicAmountSol: 1 });
    const feeUtxo = voter.getAllUtxos()[0];

    console.log(`\n\n ----------------  Casting yes vote - vote weight ${voteWeightUtxo.appData.voteWeight} ---------------- \n\n`)

    const circuitPath = path.join("build-circuit/private-voting/privateVoting");

    const currentSlot = new BN(await provider.connection.getSlot());

    let insertedVoteWeightUtxo = voteWeightUtxo;
    await voter.provider.latestMerkleTree();

    const index = voter.provider.solMerkleTree.merkleTree.indexOf(
      insertedVoteWeightUtxo.getCommitment(POSEIDON)
    );
    insertedVoteWeightUtxo.index = index;
    const elGamalPublicKeyCircuitConverted = ElGamalUtils.pointToStringArray(
      proposerElGamalKeypair.publicKey
    );
    const publicElGamalPublicKeyX = new BN(elGamalPublicKeyCircuitConverted[0]);
    const publicElGamalPublicKeyY = new BN(elGamalPublicKeyCircuitConverted[1]);
    const voteAccountInfoPreTx = await voteProgram.account.votePda.fetch(
      votePda
    );

    const voteTransactionInput: VoteTransactionInput = {
      voteWeightUtxo: insertedVoteWeightUtxo,
      feeUtxo,
      voteParameters,
      idl: IDL,
      lookUpTables: lightProvider.lookUpTables,
      proofInputs: {
        currentSlot,
        publicElGamalPublicKeyX,
        publicElGamalPublicKeyY,
        publicOldVoteWeightNoEmphemeralKeyX: new BN(
          voteAccountInfoPreTx.encryptedNoVotes[0]
        ),
        publicOldVoteWeightNoEmphemeralKeyY: new BN(
          voteAccountInfoPreTx.encryptedNoVotes[1]
        ),
        publicOldVoteWeightYesEmphemeralKeyX: new BN(
          voteAccountInfoPreTx.encryptedYesVotes[0]
        ),
        publicOldVoteWeightYesEmphemeralKeyY: new BN(
          voteAccountInfoPreTx.encryptedYesVotes[1]
        ),
        publicOldVoteWeightNoCiphertextX: new BN(
          voteAccountInfoPreTx.encryptedNoVotes[2]
        ),
        publicOldVoteWeightNoCiphertextY: new BN(
          voteAccountInfoPreTx.encryptedNoVotes[3]
        ),
        publicOldVoteWeightYesCiphertextX: new BN(
          voteAccountInfoPreTx.encryptedYesVotes[2]
        ),
        publicOldVoteWeightYesCiphertextY: new BN(
          voteAccountInfoPreTx.encryptedYesVotes[3]
        ),
      },
      voter: voter.account,
      circuitPath,
      relayer: localTestRelayer,
      solMerkleTree: voter.provider.solMerkleTree,
      voteYes: true,
    };

    let { systemProof, pspProof, transaction, pspTransactionInput } =
      await createAndProveVoteTransaction(voteTransactionInput);
    // TODO: change once I have separated accounts from proof generation
    const nullifierPda = PublicKey.findProgramAddressSync(
      [pspTransactionInput.proofInputs.voteWeightNullifier.toBuffer("be", 32)],
      verifierProgramId
    )[0];
    pspTransactionInput.accounts.nullifierPda = nullifierPda;
    pspTransactionInput.accounts.votePda = votePda;
    const solanaTransactionInputs: SolanaTransactionInputs = {
      systemProof,
      pspProof,
      transaction,
      pspTransactionInput,
    };

    const voteTxHashes = await sendAndConfirmShieldedTransaction({
      solanaTransactionInputs,
      provider: voter.provider,
      confirmOptions: ConfirmOptions.spendable,
    });

    console.log("Vote Tx Hashes: ", voteTxHashes);

    const voteAccountInfo = await voteProgram.account.votePda.fetch(votePda);
    assert.notEqual(
      voteAccountInfo.encryptedYesVotes.toString(),
      voteAccountInfoPreTx.encryptedYesVotes.toString()
    );
    assert.notEqual(
      voteAccountInfo.encryptedNoVotes.toString(),
      voteAccountInfoPreTx.encryptedNoVotes.toString()
    );

    assert.equal(
      voteAccountInfo.encryptedYesVotes.toString(),
      [
        ...pspProof.parsedPublicInputsObject[
          "publicVoteWeightYesEmphemeralKeyX"
        ],
        ...pspProof.parsedPublicInputsObject[
          "publicVoteWeightYesEmphemeralKeyY"
        ],
        ...pspProof.parsedPublicInputsObject["publicVoteWeightYesX"],
        ...pspProof.parsedPublicInputsObject["publicVoteWeightYesY"],
      ].toString()
    );
    assert.equal(
      voteAccountInfo.encryptedNoVotes.toString(),
      [
        ...pspProof.parsedPublicInputsObject[
          "publicVoteWeightNoEmphemeralKeyX"
        ],
        ...pspProof.parsedPublicInputsObject[
          "publicVoteWeightNoEmphemeralKeyY"
        ],
        ...pspProof.parsedPublicInputsObject["publicVoteWeightNoX"],
        ...pspProof.parsedPublicInputsObject["publicVoteWeightNoY"],
      ].toString()
    );
  });

  it("test publish decrypted tally: ", async () => {
    const fetchedVotePda = await fetchAndConvertVotePda(voteProgram, votePda);
    const circuitPath = path.join("build-circuit/private-voting/publishDecryptedTally");

    const createPublishDecryptedTallyTransactionInput: PublishDecryptedTallyTransactionInput =
      {
        idl: IDL,
        // @ts-ignore: fetchedVotePda contains all the fields but typescript complains
        proofInputs: {
          ...fetchedVotePda,
        },
        secretKey: proposerElGamalKeypair.secretKey,
        circuitPath,
      };

    const { proof, publicInputs } = await createPublishDecryptedTallyProof(
      createPublishDecryptedTallyTransactionInput
    );
    const publishDecryptedTallyInstruction = await voteProgram.methods
      .publishDecryptedTally(
        publicInputs[8],
        publicInputs[9],
        proof.proofA,
        proof.proofB.flat(),
        proof.proofC
      )
      .accounts({
        signer: proposerKeypair.publicKey,
        proposal: proposalPda,
        votePda,
        systemProgram: SystemProgram.programId,
      })
      .signers([proposerKeypair])
      .instruction();
    try {
      const publishDecryptedTallyTxHash = await sendAndConfirmTransaction(
        provider.connection,
        new anchor.web3.Transaction()
          .add(ComputeBudgetProgram.setComputeUnitLimit({ units: 1_400_000 }))
          .add(publishDecryptedTallyInstruction),
        [proposerKeypair],
        {
          commitment: "confirmed",
        }
      );
      console.log(
        "Publish Decrypted Tally Tx Hash: ",
        publishDecryptedTallyTxHash
      );
    } catch (e) {
      console.log("error ", e);
    }

    const voteAccountInfo = await voteProgram.account.votePda.fetch(votePda);

    assert.equal(
      voteAccountInfo.decryptedYesVoteWeight.toString(),
      voteWeightUtxo.appData.voteWeight.toString()
    );
    assert.equal(
      voteAccountInfo.decryptedNoVoteWeight.toString(),
      new BN(0).toString()
    );
    console.log("\n\n---------- fetched decrypted tally ----------\n\n");
    console.log("decrypted yes vote weight: ", voteAccountInfo.decryptedYesVoteWeight.toString());
    console.log("decrypted no vote weight: ", voteAccountInfo.decryptedNoVoteWeight.toString());
  });
});

export type UnpackedVotePda = {
  publicElGamalPublicKeyX: BN;
  publicElGamalPublicKeyY: BN;
  publicVoteWeightNoEmphemeralKeyX: BN;
  publicVoteWeightNoEmphemeralKeyY: BN;
  publicVoteWeightYesEmphemeralKeyX: BN;
  publicVoteWeightYesEmphemeralKeyY: BN;
  publicVoteWeightNoX: BN;
  publicVoteWeightNoY: BN;
  publicVoteWeightYesX: BN;
  publicVoteWeightYesY: BN;
};

export const fetchAndConvertVotePda = async (
  voteProgram: anchor.Program<PrivateVoting>,
  votePda: PublicKey
): Promise<UnpackedVotePda> => {
  const voteAccountInfoPreTx = await voteProgram.account.votePda.fetch(votePda);
  return {
    publicElGamalPublicKeyX: new BN(
      voteAccountInfoPreTx.thresholdEncryptionPubkey[0]
    ),
    publicElGamalPublicKeyY: new BN(
      voteAccountInfoPreTx.thresholdEncryptionPubkey[1]
    ),
    publicVoteWeightNoEmphemeralKeyX: new BN(
      voteAccountInfoPreTx.encryptedNoVotes[0]
    ),
    publicVoteWeightNoEmphemeralKeyY: new BN(
      voteAccountInfoPreTx.encryptedNoVotes[1]
    ),
    publicVoteWeightYesEmphemeralKeyX: new BN(
      voteAccountInfoPreTx.encryptedYesVotes[0]
    ),
    publicVoteWeightYesEmphemeralKeyY: new BN(
      voteAccountInfoPreTx.encryptedYesVotes[1]
    ),
    publicVoteWeightNoX: new BN(voteAccountInfoPreTx.encryptedNoVotes[2]),
    publicVoteWeightNoY: new BN(voteAccountInfoPreTx.encryptedNoVotes[3]),
    publicVoteWeightYesX: new BN(voteAccountInfoPreTx.encryptedYesVotes[2]),
    publicVoteWeightYesY: new BN(voteAccountInfoPreTx.encryptedYesVotes[3]),
  };
};
