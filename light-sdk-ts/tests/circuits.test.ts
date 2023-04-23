import { assert, expect } from "chai";
const chai = require("chai");
const chaiAsPromised = require("chai-as-promised");
// Load chai-as-promised support
chai.use(chaiAsPromised);
import { SystemProgram, Keypair as SolanaKeypair } from "@solana/web3.js";
import * as anchor from "@coral-xyz/anchor";
import { it } from "mocha";
import { buildPoseidonOpt } from "circomlibjs";

import { Account } from "../src/account";
import { Utxo } from "../src/utxo";
import {
  FEE_ASSET,
  hashAndTruncateToCircuit,
  Provider as LightProvider,
  MINT,
  Relayer,
  VerifierZero,
  VerifierTwo,
  TransactionErrorCode,
  Transaction,
  TransactionParameters,
  Action,
} from "../src";
import { MerkleTree } from "../src/merkleTree/merkleTree";
import { IDL } from "./testData/mock_verifier";
import { bs58 } from "@coral-xyz/anchor/dist/cjs/utils/bytes";

process.env.ANCHOR_PROVIDER_URL = "http://127.0.0.1:8899";
process.env.ANCHOR_WALLET = process.env.HOME + "/.config/solana/id.json";

const verifiers = [
  { verifier: new VerifierZero(), isApp: false },
  { verifier: new VerifierTwo(), isApp: true },
];
let keypair: Account,
  deposit_utxo1: Utxo,
  mockPubkey,
  poseidon,
  eddsa,
  lightProvider: LightProvider,
  txParamsApp: TransactionParameters,
  txParamsPoolType: TransactionParameters,
  txParamsPoolTypeOut: TransactionParameters,
  txParamsOutApp: TransactionParameters,
  txParams: TransactionParameters,
  txParamsSol: TransactionParameters,
  paramsWithdrawal: TransactionParameters,
  appData,
  relayer: Relayer;
  let seed32 = bs58.encode(new Uint8Array(32).fill(1));

// TODO: check more specific errors in tests
describe("Masp circuit tests", () => {
  before(async () => {
    poseidon = await buildPoseidonOpt();
    keypair = new Account({ poseidon: poseidon, seed: seed32, eddsa });
    await keypair.getEddsaPublicKey();
    let depositAmount = 20_000;
    let depositFeeAmount = 10_000;
    deposit_utxo1 = new Utxo({
      poseidon: poseidon,
      assets: [FEE_ASSET, MINT],
      amounts: [new anchor.BN(depositFeeAmount), new anchor.BN(depositAmount)],
      account: keypair,
    });
    let deposit_utxoSol = new Utxo({
      poseidon: poseidon,
      assets: [FEE_ASSET, MINT],
      amounts: [new anchor.BN(depositFeeAmount), new anchor.BN(0)],
      account: keypair,
    });
    mockPubkey = SolanaKeypair.generate().publicKey;
    let mockPubkey2 = SolanaKeypair.generate().publicKey;
    let mockPubkey3 = SolanaKeypair.generate().publicKey;

    lightProvider = await LightProvider.loadMock();
    txParams = new TransactionParameters({
      outputUtxos: [deposit_utxo1],
      merkleTreePubkey: mockPubkey,
      senderSpl: mockPubkey,
      senderSol: lightProvider.wallet.publicKey,
      verifier: new VerifierZero(),
      lookUpTable: mockPubkey,
      action: Action.SHIELD,
      poseidon,
      transactionIndex: 0
    });

    txParamsSol = new TransactionParameters({
      outputUtxos: [deposit_utxoSol],
      merkleTreePubkey: mockPubkey,
      senderSpl: mockPubkey,
      senderSol: lightProvider.wallet.publicKey,
      verifier: new VerifierZero(),
      lookUpTable: mockPubkey,
      action: Action.SHIELD,
      poseidon,
      transactionIndex: 0
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
    relayer = new Relayer(
      mockPubkey3,
      mockPubkey,
      mockPubkey,
      new anchor.BN(5000),
    );
    paramsWithdrawal = new TransactionParameters({
      inputUtxos: [deposit_utxo1],
      merkleTreePubkey: mockPubkey2,
      verifier: new VerifierZero(),
      poseidon,
      recipientSpl: mockPubkey,
      recipientSol: lightProvider.wallet.publicKey,
      action: Action.UNSHIELD,
      relayer,
      transactionIndex: 0
    });
    appData = {testInput1: new anchor.BN(1), testInput2: new anchor.BN(1)};
    txParamsApp = new TransactionParameters({
      inputUtxos: [
        new Utxo({
          poseidon,
          appData,
          appDataIdl: IDL,
        }),
      ],
      merkleTreePubkey: mockPubkey,
      senderSpl: mockPubkey,
      senderSol: lightProvider.wallet.publicKey,
      verifier: new VerifierTwo(),
      lookUpTable: mockPubkey,
      action: Action.UNSHIELD,
      poseidon,
      relayer,
      transactionIndex: 0
    });
    txParamsPoolType = new TransactionParameters({
      inputUtxos: [new Utxo({ poseidon, poolType: new anchor.BN("12312") })],
      merkleTreePubkey: mockPubkey,
      senderSpl: mockPubkey,
      senderSol: lightProvider.wallet.publicKey,
      verifier: new VerifierZero(),
      lookUpTable: mockPubkey,
      action: Action.UNSHIELD,
      poseidon,
      relayer,
      transactionIndex: 0
    });
    txParamsPoolTypeOut = new TransactionParameters({
      outputUtxos: [new Utxo({ poseidon, poolType: new anchor.BN("12312") })],
      merkleTreePubkey: mockPubkey,
      senderSpl: mockPubkey,
      senderSol: lightProvider.wallet.publicKey,
      verifier: new VerifierZero(),
      lookUpTable: mockPubkey,
      action: Action.UNSHIELD,
      poseidon,
      relayer,
      transactionIndex: 0
    });
    txParamsOutApp = new TransactionParameters({
      outputUtxos: [
        new Utxo({
          poseidon,
          appData,
          appDataIdl: IDL,
        }),
      ],
      merkleTreePubkey: mockPubkey,
      senderSpl: mockPubkey,
      senderSol: lightProvider.wallet.publicKey,
      verifier: new VerifierZero(),
      lookUpTable: mockPubkey,
      action: Action.SHIELD,
      poseidon,
      // automatic encryption for app utxos is not implemented
      encryptedUtxos: new Uint8Array(256).fill(1),
      transactionIndex: 0
    });
  });

  // should pass because no non zero input utxo is provided
  it("No in utxo test invalid root", async () => {
    var tx: Transaction = new Transaction({
      provider: lightProvider,
      params: txParams,
    });
    await tx.compile();
    tx.proofInputSystem.root = new anchor.BN("123").toString();
    await tx.getProof();
  });

  it("With in utxo test invalid root", async () => {
    var tx: Transaction = new Transaction({
      provider: lightProvider,
      params: paramsWithdrawal,
    });
    await tx.compile();
    tx.proofInputSystem.root = new anchor.BN("123").toString();
    await chai.assert.isRejected(
      tx.getProof(),
      TransactionErrorCode.PROOF_GENERATION_FAILED,
    );
  });

  it("With in utxo test invalid tx integrity hash", async () => {
    var tx: Transaction = new Transaction({
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
    var tx: Transaction = new Transaction({
      provider: lightProvider,
      params: txParams,
    });
    await tx.compile();
    tx.proofInputSystem.publicMintPubkey = hashAndTruncateToCircuit(
      SolanaKeypair.generate().publicKey.toBytes(),
    );
    await chai.assert.isRejected(
      tx.getProof(),
      TransactionErrorCode.PROOF_GENERATION_FAILED,
    );
  });

  it("With in utxo test invalid publicMintPubkey", async () => {
    var tx: Transaction = new Transaction({
      provider: lightProvider,
      params: paramsWithdrawal,
    });
    await tx.compile();
    tx.proofInputSystem.publicMintPubkey = hashAndTruncateToCircuit(
      SolanaKeypair.generate().publicKey.toBytes(),
    );
    await chai.assert.isRejected(
      tx.getProof(),
      TransactionErrorCode.PROOF_GENERATION_FAILED,
    );
  });

  // should succeed because no public spl amount is provided thus mint is not checked
  it("No public spl amount test invalid publicMintPubkey", async () => {
    var tx: Transaction = new Transaction({
      provider: lightProvider,
      params: txParamsSol,
    });
    await tx.compile();
    tx.proofInputSystem.publicMintPubkey = hashAndTruncateToCircuit(
      SolanaKeypair.generate().publicKey.toBytes(),
    );
    await tx.getProof();
  });

  it("With in utxo test invalid merkle proof path elements", async () => {
    var tx: Transaction = new Transaction({
      provider: lightProvider,
      params: paramsWithdrawal,
    });
    await tx.compile();

    tx.proofInputSystem.inPathElements[0] =
      tx.provider.solMerkleTree?.merkleTree.path(1).pathElements;
    await chai.assert.isRejected(
      tx.getProof(),
      TransactionErrorCode.PROOF_GENERATION_FAILED,
    );
  });

  it("With in utxo test invalid merkle proof path index", async () => {
    var tx: Transaction = new Transaction({
      provider: lightProvider,
      params: paramsWithdrawal,
    });
    await tx.compile();

    tx.proofInputSystem.inPathIndices[0] = 1;
    await chai.assert.isRejected(
      tx.getProof(),
      TransactionErrorCode.PROOF_GENERATION_FAILED,
    );
  });

  it("With in utxo test invalid inPrivateKey", async () => {
    var tx: Transaction = new Transaction({
      provider: lightProvider,
      params: paramsWithdrawal,
    });

    await tx.compile();
    tx.proofInputSystem.inPrivateKey[0] = new anchor.BN("123").toString();

    await chai.assert.isRejected(
      tx.getProof(),
      TransactionErrorCode.PROOF_GENERATION_FAILED,
    );
  });

  it("With in utxo test invalid publicAmountSpl", async () => {
    var tx: Transaction = new Transaction({
      provider: lightProvider,
      params: paramsWithdrawal,
    });

    await tx.compile();
    tx.proofInputSystem.publicAmountSpl = new anchor.BN("123").toString();

    await chai.assert.isRejected(
      tx.getProof(),
      TransactionErrorCode.PROOF_GENERATION_FAILED,
    );
  });

  it("With in utxo test invalid publicAmountSol", async () => {
    var tx: Transaction = new Transaction({
      provider: lightProvider,
      params: paramsWithdrawal,
    });

    await tx.compile();
    tx.proofInputSystem.publicAmountSol = new anchor.BN("123").toString();

    await chai.assert.isRejected(
      tx.getProof(),
      TransactionErrorCode.PROOF_GENERATION_FAILED,
    );
  });

  it("With in utxo test invalid publicAmountSpl", async () => {
    var tx: Transaction = new Transaction({
      provider: lightProvider,
      params: txParamsSol,
    });

    await tx.compile();
    tx.proofInputSystem.publicAmountSpl = new anchor.BN("123").toString();

    await chai.assert.isRejected(
      tx.getProof(),
      TransactionErrorCode.PROOF_GENERATION_FAILED,
    );
  });

  it("With in utxo test invalid outputCommitment", async () => {
    var tx: Transaction = new Transaction({
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
    var tx: Transaction = new Transaction({
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
    var tx: Transaction = new Transaction({
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
    var tx: Transaction = new Transaction({
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
    var tx: Transaction = new Transaction({
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
    var tx: Transaction = new Transaction({
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
    var tx: Transaction = new Transaction({
      provider: lightProvider,
      params: paramsWithdrawal,
    });

    await tx.compile();
    for (var i = 0; i < 3; i++) {
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
    var tx: Transaction = new Transaction({
      provider: lightProvider,
      params: txParamsApp,
      appParams: { mock: "1231" },
    });

    await tx.compile();
    await chai.assert.isRejected(
      tx.getProof(),
      TransactionErrorCode.PROOF_GENERATION_FAILED,
    );
  });

  // this works because the system verifier does not check output utxos other than commit hashes being wellformed and the sum
  it("With out utxo test inAppDataHash", async () => {
    var tx: Transaction = new Transaction({
      provider: lightProvider,
      params: txParamsOutApp,
    });

    await tx.compile();
    await tx.getProof();
  });

  // this fails because it's inconsistent with the utxo
  it("With in utxo test invalid outAppDataHash", async () => {
    var tx: Transaction = new Transaction({
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
    var tx: Transaction = new Transaction({
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
    var tx: Transaction = new Transaction({
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
    var tx: Transaction = new Transaction({
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
    var tx: Transaction = new Transaction({
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
    var tx: Transaction = new Transaction({
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
    var tx: Transaction = new Transaction({
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
    var tx: Transaction = new Transaction({
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
    var tx: Transaction = new Transaction({
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
  before(async () => {
    poseidon = await buildPoseidonOpt();
    keypair = new Account({ poseidon: poseidon, seed: seed32, eddsa });
    await keypair.getEddsaPublicKey();
    let depositAmount = 20_000;
    let depositFeeAmount = 10_000;
    deposit_utxo1 = new Utxo({
      poseidon: poseidon,
      assets: [FEE_ASSET, MINT],
      amounts: [new anchor.BN(depositFeeAmount), new anchor.BN(depositAmount)],
      account: keypair,
    });
    let deposit_utxoSol = new Utxo({
      poseidon: poseidon,
      assets: [FEE_ASSET, MINT],
      amounts: [new anchor.BN(depositFeeAmount), new anchor.BN(0)],
      account: keypair,
    });
    mockPubkey = SolanaKeypair.generate().publicKey;
    let mockPubkey2 = SolanaKeypair.generate().publicKey;
    let mockPubkey3 = SolanaKeypair.generate().publicKey;

    lightProvider = await LightProvider.loadMock();
    txParams = new TransactionParameters({
      outputUtxos: [deposit_utxo1],
      merkleTreePubkey: mockPubkey,
      senderSpl: mockPubkey,
      senderSol: lightProvider.wallet.publicKey,
      verifier: new VerifierTwo(),
      lookUpTable: mockPubkey,
      action: Action.SHIELD,
      poseidon,
      transactionIndex: 0
    });

    relayer = new Relayer(
      mockPubkey3,
      mockPubkey,
      mockPubkey,
      new anchor.BN(5000),
    );
    txParamsApp = new TransactionParameters({
      inputUtxos: [
        new Utxo({
          poseidon,
          appData,
          appDataIdl: IDL,
        }),
      ],
      merkleTreePubkey: mockPubkey,
      senderSpl: mockPubkey,
      senderSol: lightProvider.wallet.publicKey,
      verifier: new VerifierTwo(),
      lookUpTable: mockPubkey,
      action: Action.UNSHIELD,
      poseidon,
      relayer,
      transactionIndex: 0
    });
  });

  it("No in utxo test invalid transactionHash", async () => {
    var tx: Transaction = new Transaction({
      provider: lightProvider,
      params: txParams,
      appParams: { mock: "123" },
    });
    await tx.compile();

    tx.proofInput.transactionHash = new anchor.BN("123").toString();
    await chai.assert.isRejected(
      tx.getProof(),
      TransactionErrorCode.PROOF_GENERATION_FAILED,
    );
  });

  it("No in utxo test invalid transactionHash", async () => {
    var tx: Transaction = new Transaction({
      provider: lightProvider,
      params: txParamsApp,
      appParams: { mock: "123" },
    });
    await tx.compile();
    tx.proofInput.publicAppVerifier = new anchor.BN("123").toString();
    await chai.assert.isRejected(
      tx.getProof(),
      TransactionErrorCode.PROOF_GENERATION_FAILED,
    );
  });
});
