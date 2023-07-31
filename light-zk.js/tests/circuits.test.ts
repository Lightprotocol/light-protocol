import { assert } from "chai";
const chai = require("chai");
const chaiAsPromised = require("chai-as-promised");
// Load chai-as-promised support
chai.use(chaiAsPromised);
import { Keypair as SolanaKeypair } from "@solana/web3.js";
import * as anchor from "@coral-xyz/anchor";
import { it } from "mocha";
const circomlibjs = require("circomlibjs");
const { buildPoseidonOpt } = circomlibjs;

import {
  Account,
  Utxo,
  MerkleTree,
  FEE_ASSET,
  hashAndTruncateToCircuit,
  Provider as LightProvider,
  MINT,
  Relayer,
  TransactionErrorCode,
  Transaction,
  TransactionParameters,
  Action,
  IDL_VERIFIER_PROGRAM_ZERO,
  IDL_VERIFIER_PROGRAM_TWO,
} from "../src";
import { IDL } from "./testData/mock_verifier";
import { bs58 } from "@coral-xyz/anchor/dist/cjs/utils/bytes";

process.env.ANCHOR_PROVIDER_URL = "http://127.0.0.1:8899";
process.env.ANCHOR_WALLET = process.env.HOME + "/.config/solana/id.json";

let keypair: Account,
  deposit_utxo1: Utxo,
  mockPubkey,
  poseidon,
  lightProvider: LightProvider,
  txParamsApp: TransactionParameters,
  txParamsPoolType: TransactionParameters,
  txParamsPoolTypeOut: TransactionParameters,
  txParamsOutApp: TransactionParameters,
  txParams: TransactionParameters,
  txParamsSol: TransactionParameters,
  paramsWithdrawal: TransactionParameters,
  appData: any,
  relayer: Relayer;
let seed32 = bs58.encode(new Uint8Array(32).fill(1));

// TODO: check more specific errors in tests
describe("Masp circuit tests", () => {
  before(async () => {
    lightProvider = await LightProvider.loadMock();
    poseidon = await buildPoseidonOpt();
    keypair = new Account({ poseidon: poseidon, seed: seed32 });
    await keypair.getEddsaPublicKey();
    let depositAmount = 20_000;
    let depositFeeAmount = 10_000;
    deposit_utxo1 = new Utxo({
      poseidon: poseidon,
      assets: [FEE_ASSET, MINT],
      amounts: [new anchor.BN(depositFeeAmount), new anchor.BN(depositAmount)],
      account: keypair,
      assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
      verifierProgramLookupTable:
        lightProvider.lookUpTables.verifierProgramLookupTable,
    });
    let deposit_utxoSol = new Utxo({
      poseidon: poseidon,
      assets: [FEE_ASSET, MINT],
      amounts: [new anchor.BN(depositFeeAmount), new anchor.BN(0)],
      account: keypair,
      assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
      verifierProgramLookupTable:
        lightProvider.lookUpTables.verifierProgramLookupTable,
    });
    mockPubkey = SolanaKeypair.generate().publicKey;
    let mockPubkey2 = SolanaKeypair.generate().publicKey;
    let mockPubkey3 = SolanaKeypair.generate().publicKey;

    txParams = new TransactionParameters({
      outputUtxos: [deposit_utxo1],
      transactionMerkleTreePubkey: mockPubkey,
      senderSpl: mockPubkey,
      senderSol: lightProvider.wallet.publicKey,
      action: Action.SHIELD,
      poseidon,
      verifierIdl: IDL_VERIFIER_PROGRAM_ZERO,
    });

    txParamsSol = new TransactionParameters({
      outputUtxos: [deposit_utxoSol],
      transactionMerkleTreePubkey: mockPubkey,
      senderSpl: mockPubkey,
      senderSol: lightProvider.wallet.publicKey,
      action: Action.SHIELD,
      poseidon,
      verifierIdl: IDL_VERIFIER_PROGRAM_ZERO,
    });
    lightProvider.solMerkleTree!.merkleTree = new MerkleTree(18, poseidon, [
      deposit_utxo1.getCommitment(poseidon),
      // random invalid other commitment
      poseidon.F.toString(poseidon(["123124"])),
    ]);

    assert.equal(
      lightProvider.solMerkleTree?.merkleTree.indexOf(
        deposit_utxo1.getCommitment(poseidon),
      ),
      0,
    );
    relayer = new Relayer(mockPubkey3, mockPubkey, new anchor.BN(5000));
    paramsWithdrawal = new TransactionParameters({
      inputUtxos: [deposit_utxo1],
      transactionMerkleTreePubkey: mockPubkey2,
      poseidon,
      recipientSpl: mockPubkey,
      recipientSol: lightProvider.wallet.publicKey,
      action: Action.UNSHIELD,
      relayer,
      verifierIdl: IDL_VERIFIER_PROGRAM_ZERO,
    });
    appData = { testInput1: new anchor.BN(1), testInput2: new anchor.BN(1) };
    txParamsApp = new TransactionParameters({
      inputUtxos: [
        new Utxo({
          poseidon,
          appData,
          appDataIdl: IDL,
          assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
          verifierProgramLookupTable:
            lightProvider.lookUpTables.verifierProgramLookupTable,
        }),
      ],
      transactionMerkleTreePubkey: mockPubkey,
      senderSpl: mockPubkey,
      senderSol: lightProvider.wallet.publicKey,
      action: Action.UNSHIELD,
      poseidon,
      relayer,
      verifierIdl: IDL_VERIFIER_PROGRAM_TWO,
    });
    txParamsPoolType = new TransactionParameters({
      inputUtxos: [
        new Utxo({
          poseidon,
          poolType: new anchor.BN("12312"),
          assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
          verifierProgramLookupTable:
            lightProvider.lookUpTables.verifierProgramLookupTable,
        }),
      ],
      transactionMerkleTreePubkey: mockPubkey,
      senderSpl: mockPubkey,
      senderSol: lightProvider.wallet.publicKey,
      action: Action.UNSHIELD,
      poseidon,
      relayer,
      verifierIdl: IDL_VERIFIER_PROGRAM_ZERO,
    });
    txParamsPoolTypeOut = new TransactionParameters({
      outputUtxos: [
        new Utxo({
          poseidon,
          poolType: new anchor.BN("12312"),
          assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
          verifierProgramLookupTable:
            lightProvider.lookUpTables.verifierProgramLookupTable,
        }),
      ],
      transactionMerkleTreePubkey: mockPubkey,
      senderSpl: mockPubkey,
      senderSol: lightProvider.wallet.publicKey,
      action: Action.UNSHIELD,
      poseidon,
      relayer,
      verifierIdl: IDL_VERIFIER_PROGRAM_ZERO,
    });
    txParamsOutApp = new TransactionParameters({
      outputUtxos: [
        new Utxo({
          poseidon,
          appData,
          appDataIdl: IDL,
          assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
          verifierProgramLookupTable:
            lightProvider.lookUpTables.verifierProgramLookupTable,
        }),
      ],
      transactionMerkleTreePubkey: mockPubkey,
      senderSpl: mockPubkey,
      senderSol: lightProvider.wallet.publicKey,
      action: Action.SHIELD,
      poseidon,
      // automatic encryption for app utxos is not implemented
      encryptedUtxos: new Uint8Array(256).fill(1),
      verifierIdl: IDL_VERIFIER_PROGRAM_ZERO,
    });
  });

  // should pass because no non-zero input utxo is provided
  it("No in utxo test invalid root", async () => {
    let tx: Transaction = new Transaction({
      provider: lightProvider,
      params: txParams,
    });
    await tx.compile();
    tx.proofInput.root = new anchor.BN("123").toString();

    await tx.getProof();
  });

  it("With in utxo test invalid root", async () => {
    let tx: Transaction = new Transaction({
      provider: lightProvider,
      params: paramsWithdrawal,
    });
    await tx.compile();
    tx.proofInput.root = new anchor.BN("123").toString();
    await chai.assert.isRejected(
      tx.getProof(),
      TransactionErrorCode.PROOF_GENERATION_FAILED,
    );
  });

  it("With in utxo test invalid tx integrity hash", async () => {
    let tx: Transaction = new Transaction({
      provider: lightProvider,
      params: paramsWithdrawal,
    });
    await tx.compile();

    tx.proofInput.txIntegrityHash = new anchor.BN("123").toString();

    await chai.assert.isRejected(
      tx.getProof(),
      TransactionErrorCode.PROOF_GENERATION_FAILED,
    );
  });

  it("No in utxo test invalid publicMintPubkey", async () => {
    let tx: Transaction = new Transaction({
      provider: lightProvider,
      params: txParams,
    });
    await tx.compile();
    tx.proofInput.publicMintPubkey = hashAndTruncateToCircuit(
      SolanaKeypair.generate().publicKey.toBytes(),
    );
    await chai.assert.isRejected(
      tx.getProof(),
      TransactionErrorCode.PROOF_GENERATION_FAILED,
    );
  });

  it("With in utxo test invalid publicMintPubkey", async () => {
    let tx: Transaction = new Transaction({
      provider: lightProvider,
      params: paramsWithdrawal,
    });
    await tx.compile();
    tx.proofInput.publicMintPubkey = hashAndTruncateToCircuit(
      SolanaKeypair.generate().publicKey.toBytes(),
    );
    await chai.assert.isRejected(
      tx.getProof(),
      TransactionErrorCode.PROOF_GENERATION_FAILED,
    );
  });

  // should succeed because no public spl amount is provided thus mint is not checked
  it("No public spl amount test invalid publicMintPubkey", async () => {
    let tx: Transaction = new Transaction({
      provider: lightProvider,
      params: txParamsSol,
    });
    await tx.compile();
    tx.proofInput.publicMintPubkey = hashAndTruncateToCircuit(
      SolanaKeypair.generate().publicKey.toBytes(),
    );
    await tx.getProof();
  });

  it("With in utxo test invalid merkle proof path elements", async () => {
    let tx: Transaction = new Transaction({
      provider: lightProvider,
      params: paramsWithdrawal,
    });
    await tx.compile();

    tx.proofInput.inPathElements[0] =
      tx.provider.solMerkleTree?.merkleTree.path(1).pathElements;
    await chai.assert.isRejected(
      tx.getProof(),
      TransactionErrorCode.PROOF_GENERATION_FAILED,
    );
  });

  it("With in utxo test invalid merkle proof path index", async () => {
    let tx: Transaction = new Transaction({
      provider: lightProvider,
      params: paramsWithdrawal,
    });
    await tx.compile();

    tx.proofInput.inPathIndices[0] = 1;
    await chai.assert.isRejected(
      tx.getProof(),
      TransactionErrorCode.PROOF_GENERATION_FAILED,
    );
  });

  it("With in utxo test invalid inPrivateKey", async () => {
    let tx: Transaction = new Transaction({
      provider: lightProvider,
      params: paramsWithdrawal,
    });

    await tx.compile();
    tx.proofInput.inPrivateKey[0] = new anchor.BN("123").toString();

    await chai.assert.isRejected(
      tx.getProof(),
      TransactionErrorCode.PROOF_GENERATION_FAILED,
    );
  });

  it("With in utxo test invalid publicAmountSpl", async () => {
    let tx: Transaction = new Transaction({
      provider: lightProvider,
      params: paramsWithdrawal,
    });

    await tx.compile();
    tx.proofInput.publicAmountSpl = new anchor.BN("123").toString();

    await chai.assert.isRejected(
      tx.getProof(),
      TransactionErrorCode.PROOF_GENERATION_FAILED,
    );
  });

  it("With in utxo test invalid publicAmountSol", async () => {
    let tx: Transaction = new Transaction({
      provider: lightProvider,
      params: paramsWithdrawal,
    });

    await tx.compile();
    tx.proofInput.publicAmountSol = new anchor.BN("123").toString();

    await chai.assert.isRejected(
      tx.getProof(),
      TransactionErrorCode.PROOF_GENERATION_FAILED,
    );
  });

  it("With in utxo test invalid publicAmountSpl", async () => {
    let tx: Transaction = new Transaction({
      provider: lightProvider,
      params: txParamsSol,
    });

    await tx.compile();
    tx.proofInput.publicAmountSpl = new anchor.BN("123").toString();

    await chai.assert.isRejected(
      tx.getProof(),
      TransactionErrorCode.PROOF_GENERATION_FAILED,
    );
  });

  it("With in utxo test invalid outputCommitment", async () => {
    let tx: Transaction = new Transaction({
      provider: lightProvider,
      params: paramsWithdrawal,
    });

    await tx.compile();
    console.log();

    tx.proofInput.outputCommitment[0] = new anchor.BN("123").toString();

    await chai.assert.isRejected(
      tx.getProof(),
      TransactionErrorCode.PROOF_GENERATION_FAILED,
    );
  });

  it("With in utxo test invalid inAmount", async () => {
    let tx: Transaction = new Transaction({
      provider: lightProvider,
      params: paramsWithdrawal,
    });

    await tx.compile();
    tx.proofInput.inAmount[0] = new anchor.BN("123").toString();

    await chai.assert.isRejected(
      tx.getProof(),
      TransactionErrorCode.PROOF_GENERATION_FAILED,
    );
  });

  it("With in utxo test invalid outAmount", async () => {
    let tx: Transaction = new Transaction({
      provider: lightProvider,
      params: paramsWithdrawal,
    });

    await tx.compile();
    tx.proofInput.outAmount[0] = new anchor.BN("123").toString();

    await chai.assert.isRejected(
      tx.getProof(),
      TransactionErrorCode.PROOF_GENERATION_FAILED,
    );
  });

  it("With in utxo test invalid inBlinding", async () => {
    let tx: Transaction = new Transaction({
      provider: lightProvider,
      params: paramsWithdrawal,
    });

    await tx.compile();
    tx.proofInput.inBlinding[0] = new anchor.BN("123").toString();

    await chai.assert.isRejected(
      tx.getProof(),
      TransactionErrorCode.PROOF_GENERATION_FAILED,
    );
  });

  it("With in utxo test invalid outBlinding", async () => {
    let tx: Transaction = new Transaction({
      provider: lightProvider,
      params: paramsWithdrawal,
    });

    await tx.compile();
    tx.proofInput.outBlinding[0] = new anchor.BN("123").toString();

    await chai.assert.isRejected(
      tx.getProof(),
      TransactionErrorCode.PROOF_GENERATION_FAILED,
    );
  });

  it("With in utxo test invalid outPubkey", async () => {
    let tx: Transaction = new Transaction({
      provider: lightProvider,
      params: paramsWithdrawal,
    });

    await tx.compile();
    tx.proofInput.outPubkey[0] = new anchor.BN("123").toString();

    await chai.assert.isRejected(
      tx.getProof(),
      TransactionErrorCode.PROOF_GENERATION_FAILED,
    );
  });

  it("With in utxo test invalid assetPubkeys", async () => {
    let tx: Transaction = new Transaction({
      provider: lightProvider,
      params: paramsWithdrawal,
    });

    await tx.compile();
    for (let i = 0; i < 3; i++) {
      tx.proofInput.assetPubkeys[i] = hashAndTruncateToCircuit(
        SolanaKeypair.generate().publicKey.toBytes(),
      );

      await chai.assert.isRejected(
        tx.getProof(),
        TransactionErrorCode.PROOF_GENERATION_FAILED,
      );
    }
  });

  // this fails because the system verifier does not allow
  it("With in utxo test invalid inAppDataHash", async () => {
    let tx: Transaction = new Transaction({
      provider: lightProvider,
      params: txParamsApp,
      appParams: { mock: "1231", verifierIdl: IDL_VERIFIER_PROGRAM_ZERO },
    });

    await tx.compile();
    await chai.assert.isRejected(
      tx.getProof(),
      TransactionErrorCode.PROOF_GENERATION_FAILED,
    );
  });

  // this works because the system verifier does not check output utxos other than commit hashes being well-formed and the sum
  it("With out utxo test inAppDataHash", async () => {
    let tx: Transaction = new Transaction({
      provider: lightProvider,
      params: txParamsOutApp,
    });

    await tx.compile();
    await tx.getProof();
  });

  // this fails because it's inconsistent with the utxo
  it("With in utxo test invalid outAppDataHash", async () => {
    let tx: Transaction = new Transaction({
      provider: lightProvider,
      params: paramsWithdrawal,
    });

    await tx.compile();
    tx.proofInput.outAppDataHash[0] = new anchor.BN("123").toString();

    await chai.assert.isRejected(
      tx.getProof(),
      TransactionErrorCode.PROOF_GENERATION_FAILED,
    );
  });

  it("With in utxo test invalid pooltype", async () => {
    let tx: Transaction = new Transaction({
      provider: lightProvider,
      params: txParamsPoolType,
    });

    await tx.compile();
    await chai.assert.isRejected(
      tx.getProof(),
      TransactionErrorCode.PROOF_GENERATION_FAILED,
    );
  });

  it("With out utxo test invalid pooltype", async () => {
    let tx: Transaction = new Transaction({
      provider: lightProvider,
      params: txParamsPoolTypeOut,
    });

    await tx.compile();
    await chai.assert.isRejected(
      tx.getProof(),
      TransactionErrorCode.PROOF_GENERATION_FAILED,
    );
  });

  it("With in utxo test invalid inPoolType", async () => {
    let tx: Transaction = new Transaction({
      provider: lightProvider,
      params: paramsWithdrawal,
    });

    await tx.compile();
    tx.proofInput.inPoolType[0] = new anchor.BN("123").toString();

    await chai.assert.isRejected(
      tx.getProof(),
      TransactionErrorCode.PROOF_GENERATION_FAILED,
    );
  });

  it("With in utxo test invalid outPoolType", async () => {
    let tx: Transaction = new Transaction({
      provider: lightProvider,
      params: paramsWithdrawal,
    });

    await tx.compile();
    tx.proofInput.outPoolType[0] = new anchor.BN("123").toString();

    await chai.assert.isRejected(
      tx.getProof(),
      TransactionErrorCode.PROOF_GENERATION_FAILED,
    );
  });

  it("With in utxo test invalid inIndices", async () => {
    let tx: Transaction = new Transaction({
      provider: lightProvider,
      params: paramsWithdrawal,
    });

    await tx.compile();

    tx.proofInput.inIndices[0][0][0] = new anchor.BN("123").toString();

    await chai.assert.isRejected(
      tx.getProof(),
      TransactionErrorCode.PROOF_GENERATION_FAILED,
    );
  });

  it("With in utxo test invalid inIndices", async () => {
    let tx: Transaction = new Transaction({
      provider: lightProvider,
      params: paramsWithdrawal,
    });

    await tx.compile();
    chai.assert.notEqual(tx.proofInput.outIndices[1][1][1].toString(), "1");
    tx.proofInput.inIndices[1][1][1] = "1";

    await chai.assert.isRejected(
      tx.getProof(),
      TransactionErrorCode.PROOF_GENERATION_FAILED,
    );
  });

  it("With in utxo test invalid outIndices", async () => {
    let tx: Transaction = new Transaction({
      provider: lightProvider,
      params: paramsWithdrawal,
    });

    await tx.compile();

    tx.proofInput.outIndices[0][0][0] = new anchor.BN("123").toString();

    await chai.assert.isRejected(
      tx.getProof(),
      TransactionErrorCode.PROOF_GENERATION_FAILED,
    );
  });

  it("With in utxo test invalid outIndices", async () => {
    let tx: Transaction = new Transaction({
      provider: lightProvider,
      params: paramsWithdrawal,
    });

    await tx.compile();
    chai.assert.notEqual(tx.proofInput.outIndices[1][1][1].toString(), "1");
    tx.proofInput.outIndices[1][1][1] = "1";

    await chai.assert.isRejected(
      tx.getProof(),
      TransactionErrorCode.PROOF_GENERATION_FAILED,
    );
  });
});

// TODO: check more specific errors in tests
describe("App system circuit tests", () => {
  let lightProvider: LightProvider;
  before(async () => {
    lightProvider = await LightProvider.loadMock();
    poseidon = await buildPoseidonOpt();
    keypair = new Account({ poseidon: poseidon, seed: seed32 });
    await keypair.getEddsaPublicKey();
    let depositAmount = 20_000;
    let depositFeeAmount = 10_000;
    deposit_utxo1 = new Utxo({
      poseidon: poseidon,
      assets: [FEE_ASSET, MINT],
      amounts: [new anchor.BN(depositFeeAmount), new anchor.BN(depositAmount)],
      account: keypair,
      assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
      verifierProgramLookupTable:
        lightProvider.lookUpTables.verifierProgramLookupTable,
    });
    mockPubkey = SolanaKeypair.generate().publicKey;
    let relayerPubkey = SolanaKeypair.generate().publicKey;

    lightProvider = await LightProvider.loadMock();
    txParams = new TransactionParameters({
      outputUtxos: [deposit_utxo1],
      transactionMerkleTreePubkey: mockPubkey,
      senderSpl: mockPubkey,
      senderSol: lightProvider.wallet.publicKey,
      action: Action.SHIELD,
      poseidon,
      verifierIdl: IDL_VERIFIER_PROGRAM_TWO,
    });

<<<<<<< HEAD
    relayer = new Relayer(
      relayerPubkey,
      mockPubkey,
      mockPubkey,
      new anchor.BN(5000),
    );
=======
    relayer = new Relayer(relayerPubkey, mockPubkey, new anchor.BN(5000));
>>>>>>> main
    txParamsApp = new TransactionParameters({
      inputUtxos: [
        new Utxo({
          poseidon,
          appData,
          appDataIdl: IDL,
          assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
          verifierProgramLookupTable:
            lightProvider.lookUpTables.verifierProgramLookupTable,
        }),
      ],
      transactionMerkleTreePubkey: mockPubkey,
      senderSpl: mockPubkey,
      senderSol: lightProvider.wallet.publicKey,
      action: Action.UNSHIELD,
      poseidon,
      relayer,
      verifierIdl: IDL_VERIFIER_PROGRAM_TWO,
    });
  });

  it("No in utxo test invalid transactionHash", async () => {
    let tx: Transaction = new Transaction({
      provider: lightProvider,
      params: txParams,
      appParams: { mock: "123", verifierIdl: IDL_VERIFIER_PROGRAM_ZERO },
    });
    await tx.compile();

    tx.proofInput.transactionHash = new anchor.BN("123").toString();
    await chai.assert.isRejected(
      tx.getProof(),
      TransactionErrorCode.PROOF_GENERATION_FAILED,
    );
  });

  it("No in utxo test invalid transactionHash", async () => {
    let tx: Transaction = new Transaction({
      provider: lightProvider,
      params: txParamsApp,
      appParams: { mock: "123", verifierIdl: IDL_VERIFIER_PROGRAM_ZERO },
    });
    await tx.compile();
    tx.proofInput.publicAppVerifier = new anchor.BN("123").toString();
    await chai.assert.isRejected(
      tx.getProof(),
      TransactionErrorCode.PROOF_GENERATION_FAILED,
    );
  });
});
