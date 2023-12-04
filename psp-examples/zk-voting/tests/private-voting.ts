import * as anchor from "@coral-xyz/anchor";
import { assert } from "chai";
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
  PspTransactionInput,
  BN_0,
  TransactionParameters,
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
import { IDL as VOTE_WEIGHT_IDL } from "../target/types/vote_weight_program";

import {
  decode,
  decrypt,
  ElGamalUtils,
  generateKeypair,
} from "@lightprotocol/circuit-lib.js";
import {
  claimVoteWeightUtxoTransactionInput,
  createAndProveClaimVoteUtxoTransaction,
  createAndProveCreateVoteUtxoTransaction,
  createVoteWeightUtxoTransactionInput,
  createInitVoteProof,
  createAndProveVoteTransaction,
  VoteParameters,
  VoteTransactionInput,
  VoteWeightUtxoData,
  fetchAndConvertVotePda,
  createPublishDecryptedTallyProof,
  PublishDecryptedTallyTransactionInput,
} from "../sdk/index";
const path = require("path");
const { coordinatesToExtPoint } = ElGamalUtils;

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
  let voteWeightConfig, voteWeightConfigPda: PublicKey;
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
    proposerElGamalKeypair: circuitlibjs.Keypair,
    createVoteUtxoTransactionInput: PspTransactionInput;

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
    console.log(
      `\n\n ----------------  Creating mock proposal ---------------- \n\n`
    );

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
      startSlot: new BN(0),
      releaseSlot: new BN(0),
      rate: new BN(0),
      voteLock: new BN(0),
      voteUtxoNumber: new BN(0),
      voteUtxoIdNonce: new BN(0),
      // TODO: once we have a separate vote weight psp program, we can use that here.
      voteWeightPspAddress: hashAndTruncateToCircuit(
        verifierProgramId.toBytes()
      ),
    };
    console.log(
      `\n\n ----------------  Creating vote weight utxo: vote weight ${voteWeightUtxoData.voteWeight} ---------------- \n\n`
    );

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
      confirmOptions: ConfirmOptions.spendable,
    });

    console.log(
      "store program utxo transaction hash ",
      storeTransaction.txHash
    );
    // const programUtxoBalanceOld: Map<string, ProgramUtxoBalance> =
    // await voter.syncStorage(IDL);
    // console.log("program utxo balance old ", programUtxoBalanceOld);
    const programUtxoBalance: Map<string, ProgramUtxoBalance> =
      await voter.syncStorage(IDL);
    console.log("commitment ", voteWeightUtxo.getCommitment(POSEIDON));
    console.log(
      "program utxo balance ",
      programUtxoBalance
        .get(TransactionParameters.getVerifierProgramId(IDL).toBase58())
        .tokenBalances.get(SystemProgram.programId.toBase58())
        .utxos.values()
    );
    const shieldedUtxoCommitmentHash = voteWeightUtxo.getCommitment(POSEIDON);
    const inputUtxo = Array.from(
      programUtxoBalance
        .get(TransactionParameters.getVerifierProgramId(IDL).toBase58())
        .tokenBalances.get(SystemProgram.programId.toBase58())
        .utxos.values()
    )[0]; //.get(shieldedUtxoCommitmentHash);
    Utxo.equal(POSEIDON, inputUtxo, voteWeightUtxo, true);
  });

  /**
   * 1. Set up pda with vote weight utxo parameters
   * 2. Create vote weight utxo
   */

  it("create vote weight config ", async () => {
    voteWeightConfig = {
      governingTokenMint: SystemProgram.programId,
      voteUtxoNumber: new BN(0),
      publicMaxLockTime: new BN(101),
    };
    const voteWeightConfigAuthority = Keypair.generate();
    await airdropSol({
      connection: provider.connection,
      lamports: 1e10,
      recipientPublicKey: voteWeightConfigAuthority.publicKey,
    });

    const programId =
      TransactionParameters.getVerifierProgramId(VOTE_WEIGHT_IDL);

    console.log(
      `\n\n ----------------  Creating vote weight config ---------------- \n\n`
    );

    // TODO: add pda derivation function to anchor
    voteWeightConfigPda = PublicKey.findProgramAddressSync(
      [voteWeightConfig.governingTokenMint.toBuffer()],
      programId
    )[0];
    const voteWeightProgram = new anchor.Program(VOTE_WEIGHT_IDL, programId);
    const instruction = await voteWeightProgram.methods
      .initVoteWeightConfig(voteWeightConfig.publicMaxLockTime)
      .accounts({
        signingAddress: voteWeightConfigAuthority.publicKey,
        voteWeightConfig: voteWeightConfigPda,
        systemProgram: SystemProgram.programId,
        governanceTokenMint: voteWeightConfig.governingTokenMint,
      })
      .signers([voteWeightConfigAuthority])
      .instruction();

    const initVoteWeightConfigTxHash = await sendAndConfirmTransaction(
      provider.connection,
      new anchor.web3.Transaction().add(instruction),
      [voteWeightConfigAuthority],
      {
        commitment: "confirmed",
      }
    );
    console.log(
      "Init Vote Weight Config Tx Hash: ",
      initVoteWeightConfigTxHash
    );

    const voteWeightConfigAccountInfo =
      await voteWeightProgram.account.voteWeightConfig.fetch(
        voteWeightConfigPda
      );
    assert.equal(
      voteWeightConfigAccountInfo.governanceTokenMint.toBase58(),
      voteWeightConfig.governingTokenMint.toBase58()
    );
    assert.equal(
      voteWeightConfigAccountInfo.currentVoteWeightNumber.toString(),
      "0"
    );
    assert.equal(
      voteWeightConfigAccountInfo.maxLockTime.toString(),
      voteWeightConfig.publicMaxLockTime.toString()
    );
    assert.equal(
      voteWeightConfigAccountInfo.authority.toBase58(),
      voteWeightConfigAuthority.publicKey.toBase58()
    );
    // const instructionPrint = await  voteWeightProgram.methods.verifyCreateVoteWeightProofInstruction(Buffer.alloc(256).fill(1),[new Array(32)]).accounts({
    //   voteWeightConfig: voteWeightConfigPda,
    // }).instruction();

    // console.log("instruction discriminator ", Array.from(instructionPrint.data.slice(0, 8)).toString());
  });

  it("create vote weight utxo (not used yet)", async () => {
    await voter.shield({ token: "SOL", publicAmountSol: 3 });

    await voter.getBalance();
    let inUtxo = voter.getAllUtxos()[0];
    const circuitPath = path.join(
      "build-circuit/vote-weight-program/createVoteUtxo"
    );

    const createVoteWeightUtxoTransactionInput: createVoteWeightUtxoTransactionInput =
      {
        inUtxos: [inUtxo],
        // feeUtxo,
        idl: IDL,
        pspIdl: VOTE_WEIGHT_IDL,
        lookUpTables: lightProvider.lookUpTables,
        voter: voter.account,
        circuitPath,
        relayer: localTestRelayer,
        solMerkleTree: lightProvider.solMerkleTree,
        timeLocked: new BN(1),
        voteUtxoNumber: BN_0,
        voteWeightCreationParamatersPda: {
          governingTokenMint: SystemProgram.programId,
          voteUtxoNumber: BN_0,
          publicMaxLockTime: new BN(101),
        },
        publicCurrentSlot: new BN(await provider.connection.getSlot()),
        voteWeightAmount: new BN(1e9),
        verifierProgramId,
        voteWeightConfig: voteWeightConfigPda,
        voteWeightProgramId:
          TransactionParameters.getVerifierProgramId(VOTE_WEIGHT_IDL),
      };
    let { systemProof, pspProof, pspTransactionInput, transaction } =
      await createAndProveCreateVoteUtxoTransaction(
        createVoteWeightUtxoTransactionInput,
        voter.provider.poseidon
      );
    // save for reuse in claim vote weight utxo
    createVoteUtxoTransactionInput = pspTransactionInput;
    pspTransactionInput.verifierIdl = IDL;
    const solanaTransactionInputs: SolanaTransactionInputs = {
      systemProof,
      pspProof,
      transaction,
      pspTransactionInput,
      prefix: "createVoteWeight",
    };

    const voteTxHashes = await sendAndConfirmShieldedTransaction({
      solanaTransactionInputs,
      provider: voter.provider,
      confirmOptions: ConfirmOptions.spendable,
    });
    console.log("Vote Tx Hashes: ", voteTxHashes);

    await voter.getBalance();

    assert(
      voter.provider.solMerkleTree.merkleTree.indexOf(
        createVoteUtxoTransactionInput.checkedOutUtxos[0].utxo.getCommitment(
          POSEIDON
        )
      ) != -1
    );
    // program utxo is not stored yet thus we cannot assert that it is fetched correctly
    // TODO: add fetched check once verifier 4in4out storage capability is implemented
  });

  it("claim vote weight utxo ", async () => {
    await voter.getBalance();
    let feeUtxo = voter.getAllUtxos()[0];

    let voteWeightUtxo = createVoteUtxoTransactionInput.checkedOutUtxos[0].utxo;
    voteWeightUtxo.index = voter.provider.solMerkleTree.merkleTree.indexOf(
      voteWeightUtxo.getCommitment(POSEIDON)
    );
    const circuitPath = path.join(
      "build-circuit/vote-weight-program/createVoteUtxo"
    );

    const createVoteWeightUtxoTransactionInput: claimVoteWeightUtxoTransactionInput =
      {
        voteWeightUtxo,
        feeUtxo,
        idl: IDL,
        pspIdl: VOTE_WEIGHT_IDL,
        lookUpTables: lightProvider.lookUpTables,
        voter: voter.account,
        circuitPath,
        relayer: localTestRelayer,
        solMerkleTree: lightProvider.solMerkleTree,
        voteWeightCreationParamatersPda: {
          governingTokenMint: SystemProgram.programId,
          voteUtxoNumber: BN_0,
          publicMaxLockTime: new BN(101),
        },
        publicCurrentSlot: new BN(await provider.connection.getSlot()),
        verifierProgramId,
        voteWeightConfig: voteWeightConfigPda,
        voteWeightProgramId:
          TransactionParameters.getVerifierProgramId(VOTE_WEIGHT_IDL),
      };
    let { systemProof, pspProof, pspTransactionInput, transaction } =
      await createAndProveClaimVoteUtxoTransaction(
        createVoteWeightUtxoTransactionInput,
        voter.provider.poseidon
      );

    pspTransactionInput.verifierIdl = IDL;
    const solanaTransactionInputs: SolanaTransactionInputs = {
      systemProof,
      pspProof,
      transaction,
      pspTransactionInput,
      prefix: "createVoteWeight",
    };

    const voteTxHashes = await sendAndConfirmShieldedTransaction({
      solanaTransactionInputs,
      provider: voter.provider,
      confirmOptions: ConfirmOptions.spendable,
    });
    console.log("Vote Tx Hashes: ", voteTxHashes);
    let balance = await voter.getBalance();
    assert.equal(
      balance.tokenBalances.get(SystemProgram.programId.toBase58()).utxos.size,
      2
    );
  });

  it("init vote: ", async () => {
    votePda = PublicKey.findProgramAddressSync(
      [proposalPda.toBuffer(), utils.bytes.utf8.encode("VOTE")],
      verifierProgramId
    )[0];
    console.log(
      `\n\n ----------------  Creating vote pda ${votePda.toBase58()} ---------------- \n\n`
    );

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
    console.log(`\n\n ----------------  Shielding 1 sol ---------------- \n\n`);

    // create fee utxo
    await voter.shield({ token: "SOL", publicAmountSol: 1 });
    const feeUtxo = voter.getAllUtxos()[0];

    console.log(
      `\n\n ----------------  Casting yes vote - vote weight ${voteWeightUtxo.appData.voteWeight} ---------------- \n\n`
    );

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
      await createAndProveVoteTransaction(
        voteTransactionInput,
        voter.provider.poseidon
      );
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
      prefix: "light",
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
    const circuitPath = path.join(
      "build-circuit/private-voting/publishDecryptedTally"
    );

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
    console.log(
      "decrypted yes vote weight: ",
      voteAccountInfo.decryptedYesVoteWeight.toString()
    );
    console.log(
      "decrypted no vote weight: ",
      voteAccountInfo.decryptedNoVoteWeight.toString()
    );
  });
});
