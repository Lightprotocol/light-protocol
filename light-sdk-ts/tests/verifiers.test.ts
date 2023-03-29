import { assert, expect } from "chai";
const chai = require("chai");
const chaiAsPromised = require("chai-as-promised");
const should = chai.should();
// Load chai-as-promised support
chai.use(chaiAsPromised);
import { SystemProgram, Keypair as SolanaKeypair } from "@solana/web3.js";
import * as anchor from "@coral-xyz/anchor";
import { it } from "mocha";
import { buildPoseidonOpt, buildEddsa } from "circomlibjs";

import { Account } from "../src/account";
import { Utxo } from "../src/utxo";
import {
  FEE_ASSET,
  hashAndTruncateToCircuit,
  Provider as LightProvider,
  MINT,
  Relayer,
  UtxoError,
  UtxoErrorCode,
  functionalCircuitTest,
  VerifierZero,
  VerifierTwo,
  VerifierOne,
  VerifierError,
  VerifierErrorCode,
  TransactionErrorCode,
  TransactionError,
  TransactioParametersError,
  TransactionParametersErrorCode,
  Transaction,
  TransactionParameters,
  Action,
} from "../src";
import { MerkleTree } from "../src/merkleTree/merkleTree";
process.env.ANCHOR_PROVIDER_URL = "http://127.0.0.1:8899";
process.env.ANCHOR_WALLET = process.env.HOME + "/.config/solana/id.json";

const verifiers = [
  // { verifier: new VerifierZero(), isApp: false },
  { verifier: new VerifierOne(), isApp: false },
  { verifier: new VerifierTwo(), isApp: true },
];

// TODO: add circuit tests
// for every public input pass in a wrong input
// for every private input pass in a wrong input
describe("Verifier tests", () => {
  let poseidon, eddsa,lightProvider: LightProvider, txParams: TransactionParameters,txParamsSol: TransactionParameters, paramsWithdrawal: TransactionParameters, relayer: Relayer, ;

  before(async () => {
    poseidon = await buildPoseidonOpt();
    eddsa = await buildEddsa();
    let seed32 = new Uint8Array(32).fill(1).toString();

    let keypair = new Account({ poseidon: poseidon, seed: seed32, eddsa });
    await keypair.getEddsaPublicKey();
    let depositAmount = 20_000;
    let depositFeeAmount = 10_000;
    let deposit_utxo1 = new Utxo({
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
    let mockPubkey = SolanaKeypair.generate().publicKey;
    let mockPubkey2 = SolanaKeypair.generate().publicKey;
    let mockPubkey3 = SolanaKeypair.generate().publicKey;

    lightProvider = await LightProvider.loadMock();
    txParams = new TransactionParameters({
      outputUtxos: [deposit_utxo1],
      merkleTreePubkey: mockPubkey,
      sender: mockPubkey,
      senderFee: lightProvider.nodeWallet!.publicKey,
      verifier: new VerifierZero(),
      lookUpTable: mockPubkey,
      action: Action.SHIELD,
      poseidon,
      senderShieldedAccount: keypair,
    });

    txParamsSol = new TransactionParameters({
      outputUtxos: [deposit_utxoSol],
      merkleTreePubkey: mockPubkey,
      sender: mockPubkey,
      senderFee: lightProvider.nodeWallet!.publicKey,
      verifier: new VerifierZero(),
      lookUpTable: mockPubkey,
      action: Action.SHIELD,
      poseidon,
      senderShieldedAccount: keypair,
    });
    lightProvider.solMerkleTree!.merkleTree = new MerkleTree(18, poseidon, [
      deposit_utxo1.getCommitment(),
      // random invalid other commitment
      poseidon.F.toString(poseidon(["123124"]))
    ]);

    assert.equal(
      lightProvider.solMerkleTree?.merkleTree.indexOf(
        deposit_utxo1.getCommitment(),
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
      recipient: mockPubkey,
      recipientFee: lightProvider.nodeWallet?.publicKey,
      action: Action.UNSHIELD,
      relayer,
      senderShieldedAccount: keypair
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
      TransactionErrorCode.PROOF_GENERATION_FAILED
    )
  });

  it("With in utxo test invalid tx integrity hash", async () => {
    var tx: Transaction = new Transaction({
      provider: lightProvider,
      params: paramsWithdrawal,
    });
    await tx.compile();    
    tx.proofInputSystem.extDataHash = new anchor.BN("123").toString();
    await chai.assert.isRejected(
      tx.getProof(),
      TransactionErrorCode.PROOF_GENERATION_FAILED
    )
  });

  it("No in utxo test invalid mintPubkey", async () => {
    var tx: Transaction = new Transaction({
      provider: lightProvider,
      params: txParams,
    });
    await tx.compile();    
    tx.proofInputSystem.mintPubkey = hashAndTruncateToCircuit(SolanaKeypair.generate().publicKey.toBytes());
    await chai.assert.isRejected(
      tx.getProof(),
      TransactionErrorCode.PROOF_GENERATION_FAILED
    )
  });

  it("With in utxo test invalid mintPubkey", async () => {
    var tx: Transaction = new Transaction({
      provider: lightProvider,
      params: paramsWithdrawal,
    });
    await tx.compile();    
    tx.proofInputSystem.mintPubkey = hashAndTruncateToCircuit(SolanaKeypair.generate().publicKey.toBytes());
    await chai.assert.isRejected(
      tx.getProof(),
      TransactionErrorCode.PROOF_GENERATION_FAILED
    )
  });

  // should succeed because no public spl amount is provided thus mint is not checked
  it("No public spl amount test invalid mintPubkey", async () => {
    var tx: Transaction = new Transaction({
      provider: lightProvider,
      params: txParamsSol,
    });
    await tx.compile();    
    tx.proofInputSystem.mintPubkey = hashAndTruncateToCircuit(SolanaKeypair.generate().publicKey.toBytes());
    await tx.getProof();
  });




  it("With in utxo test invalid merkle proof path elements", async () => {
    var tx: Transaction = new Transaction({
      provider: lightProvider,
      params: paramsWithdrawal,
    });
    await tx.compile();    
    
    tx.proofInputSystem.inPathElements[0] = tx.provider.solMerkleTree?.merkleTree.path(1)
    .pathElements;
    await chai.assert.isRejected(
      tx.getProof(),
      TransactionErrorCode.PROOF_GENERATION_FAILED
    )
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
      TransactionErrorCode.PROOF_GENERATION_FAILED
    )
  });

  it("With in utxo test invalid signature", async () => {
    var tx: Transaction = new Transaction({
      provider: lightProvider,
      params: paramsWithdrawal,
    });

    await tx.compile();    
    tx.proofInputSystem.signatures = new anchor.BN("123").toString();

    await chai.assert.isRejected(
      tx.getProof(),
      TransactionErrorCode.PROOF_GENERATION_FAILED
    )
  });

  // should pass because no non zero input utxo is provided
  it("No in utxo test random signer", async () => {
    var tx: Transaction = new Transaction({
      provider: lightProvider,
      params: txParams,
    });
    tx.params.senderShieldedAccount = new Account({poseidon, eddsa});
    await tx.params.senderShieldedAccount.getEddsaPublicKey();
    await tx.compile();
    await tx.getProof();
  });

  it("Wtih in utxo test random signer", async () => {
    var tx: Transaction = new Transaction({
      provider: lightProvider,
      params: paramsWithdrawal,
    });
    tx.params.senderShieldedAccount = new Account({poseidon, eddsa});
    await tx.params.senderShieldedAccount.getEddsaPublicKey();
    await tx.compile();
    await chai.assert.isRejected(
      tx.getProof(),
      TransactionErrorCode.PROOF_GENERATION_FAILED
    )
  });

  it("test invalid signature", async () => {
    
    
  });



})

describe("Verifier tests", () => {
  let poseidon;
  before(async () => {
    poseidon = await buildPoseidonOpt();
  });

  it.only("Test functional circuit", async () => {
    for (var verifier in verifiers) {
      await functionalCircuitTest(
        verifiers[verifier].verifier,
        verifiers[verifier].isApp,
      );
    }
  });

  it("Public inputs: INVALID_INPUTS_NUMBER", async () => {
    for (var verifier in verifiers) {
      expect(() => {
        verifiers[verifier].verifier.parsePublicInputsFromArray([[]]);
      })
        .throw(VerifierError)
        .includes({
          code: VerifierErrorCode.INVALID_INPUTS_NUMBER,
          functionName: "parsePublicInputsFromArray",
        });
    }
  });

  it("PUBLIC_INPUTS_UNDEFINED", async () => {
    for (var verifier in verifiers) {
      expect(() => {
        // @ts-ignore: for test
        verifiers[verifier].verifier.parsePublicInputsFromArray();
      })
        .throw(VerifierError)
        .includes({
          code: VerifierErrorCode.PUBLIC_INPUTS_UNDEFINED,
          functionName: "parsePublicInputsFromArray",
        });
    }
  });

  it("TX_PARAMETERS_UNDEFINED", async () => {
    for (var verifier in verifiers) {
      if (!verifiers[verifier].isApp) {
        await chai.assert.isRejected(
          // @ts-ignore:
          verifiers[verifier].verifier.getInstructions({}),
          TransactionErrorCode.TX_PARAMETERS_UNDEFINED,
        );
      }
    }
  });

  it("REMAINING_ACCOUNTS_NOT_CREATED", async () => {
    for (var verifier in verifiers) {
      if (!verifiers[verifier].isApp) {
        await chai.assert.isRejected(
          // @ts-ignore:
          verifiers[verifier].verifier.getInstructions({ params: {} }),
          TransactionErrorCode.REMAINING_ACCOUNTS_NOT_CREATED,
        );
      }
    }
  });

  it("REMAINING_ACCOUNTS_NOT_CREATED nullifier", async () => {
    for (var verifier in verifiers) {
      if (!verifiers[verifier].isApp) {
        await chai.assert.isRejected(
          verifiers[verifier].verifier.getInstructions({
            // @ts-ignore:
            params: {},
            remainingAccounts: {},
          }),
          // TransactionError
          TransactionErrorCode.REMAINING_ACCOUNTS_NOT_CREATED,
        );
      }
    }
  });

  it("REMAINING_ACCOUNTS_NOT_CREATED leaves", async () => {
    for (var verifier in verifiers) {
      if (!verifiers[verifier].isApp) {
        await chai.assert.isRejected(
          verifiers[verifier].verifier.getInstructions({
            // @ts-ignore:
            params: {},
            remainingAccounts: { nullifierPdaPubkeys: [] },
          }),
          TransactionErrorCode.REMAINING_ACCOUNTS_NOT_CREATED,
        );
      }
    }
  });

  it("PUBLIC_INPUTS_UNDEFINED", async () => {
    for (var verifier in verifiers) {
      if (!verifiers[verifier].isApp) {
        await chai.assert.isRejected(
          // @ts-ignore:
          verifiers[verifier].verifier.getInstructions({
            // @ts-ignore:
            params: {},
            remainingAccounts: {
              nullifierPdaPubkeys: [],
              leavesPdaPubkeys: [],
            },
            transactionInputs: {},
          }),
          TransactionErrorCode.PUBLIC_INPUTS_UNDEFINED,
        );
      }
    }
  });

  it("RELAYER_UNDEFINED", async () => {
    for (var verifier in verifiers) {
      if (!verifiers[verifier].isApp) {
        await chai.assert.isRejected(
          // @ts-ignore:
          verifiers[verifier].verifier.getInstructions({
            // @ts-ignore:
            params: {},
            remainingAccounts: {
              nullifierPdaPubkeys: [],
              leavesPdaPubkeys: [],
            },
            // @ts-ignore:
            transactionInputs: { publicInputs: [] },
          }),
          TransactionErrorCode.RELAYER_UNDEFINED,
        );
      }
    }
  });

  it("ENCRYPTING_UTXOS_UNDEFINED", async () => {
    for (var verifier in verifiers) {
      if (!verifiers[verifier].isApp) {
        await chai.assert.isRejected(
          // @ts-ignore:
          verifiers[verifier].verifier.getInstructions({
            // @ts-ignore:
            params: { relayer: {} },
            remainingAccounts: {
              nullifierPdaPubkeys: [],
              leavesPdaPubkeys: [],
            },
            // @ts-ignore:
            transactionInputs: { publicInputs: [] },
          }),
          VerifierErrorCode.ENCRYPTING_UTXOS_UNDEFINED,
        );
      }
    }
  });

  it("WALLET_UNDEFINED", async () => {
    for (var verifier in verifiers) {
      if (!verifiers[verifier].isApp) {
        await chai.assert.isRejected(
          // @ts-ignore:
          verifiers[verifier].verifier.getInstructions({
            // @ts-ignore:
            params: { relayer: {}, encryptedUtxos: new Array(1) },
            remainingAccounts: {
              nullifierPdaPubkeys: [],
              leavesPdaPubkeys: [],
            },
            // @ts-ignore:
            provider: {},
            // @ts-ignore:
            transactionInputs: { publicInputs: [] },
          }),
          TransactionErrorCode.WALLET_UNDEFINED,
        );
      }
    }
  });
});
