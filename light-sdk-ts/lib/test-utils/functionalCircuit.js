"use strict";
var __createBinding =
  (this && this.__createBinding) ||
  (Object.create
    ? function (o, m, k, k2) {
        if (k2 === undefined) k2 = k;
        var desc = Object.getOwnPropertyDescriptor(m, k);
        if (
          !desc ||
          ("get" in desc ? !m.__esModule : desc.writable || desc.configurable)
        ) {
          desc = {
            enumerable: true,
            get: function () {
              return m[k];
            },
          };
        }
        Object.defineProperty(o, k2, desc);
      }
    : function (o, m, k, k2) {
        if (k2 === undefined) k2 = k;
        o[k2] = m[k];
      });
var __setModuleDefault =
  (this && this.__setModuleDefault) ||
  (Object.create
    ? function (o, v) {
        Object.defineProperty(o, "default", { enumerable: true, value: v });
      }
    : function (o, v) {
        o["default"] = v;
      });
var __importStar =
  (this && this.__importStar) ||
  function (mod) {
    if (mod && mod.__esModule) return mod;
    var result = {};
    if (mod != null)
      for (var k in mod)
        if (k !== "default" && Object.prototype.hasOwnProperty.call(mod, k))
          __createBinding(result, mod, k);
    __setModuleDefault(result, mod);
    return result;
  };
var __awaiter =
  (this && this.__awaiter) ||
  function (thisArg, _arguments, P, generator) {
    function adopt(value) {
      return value instanceof P
        ? value
        : new P(function (resolve) {
            resolve(value);
          });
    }
    return new (P || (P = Promise))(function (resolve, reject) {
      function fulfilled(value) {
        try {
          step(generator.next(value));
        } catch (e) {
          reject(e);
        }
      }
      function rejected(value) {
        try {
          step(generator["throw"](value));
        } catch (e) {
          reject(e);
        }
      }
      function step(result) {
        result.done
          ? resolve(result.value)
          : adopt(result.value).then(fulfilled, rejected);
      }
      step((generator = generator.apply(thisArg, _arguments || [])).next());
    });
  };
Object.defineProperty(exports, "__esModule", { value: true });
exports.functionalCircuitTest = void 0;
const index_1 = require("../index");
const anchor = __importStar(require("@coral-xyz/anchor"));
const web3_js_1 = require("@solana/web3.js");
const chai_1 = require("chai");
const circomlibjs = require("circomlibjs");
function functionalCircuitTest() {
  return __awaiter(this, void 0, void 0, function* () {
    console.log("disabled following prints");
    // console.log = () => {}
    const poseidon = yield circomlibjs.buildPoseidonOpt();
    let seed32 = new Uint8Array(32).fill(1).toString();
    let keypair = new index_1.Keypair({ poseidon: poseidon, seed: seed32 });
    let depositAmount = 20000;
    let depositFeeAmount = 10000;
    console.log("MerkleTree ", new index_1.MerkleTree(18, poseidon));
    let tx = new index_1.Transaction({
      payer: index_1.ADMIN_AUTH_KEYPAIR,
      encryptionKeypair: index_1.ENCRYPTION_KEYPAIR,
      // four static config fields
      merkleTree: new index_1.MerkleTree(18, poseidon),
      provider: undefined,
      lookupTable: undefined,
      relayerRecipient: index_1.ADMIN_AUTH_KEYPAIR.publicKey,
      verifier: new index_1.VerifierZero(),
      shuffleEnabled: false,
      poseidon: poseidon,
    });
    let deposit_utxo1 = new index_1.Utxo({
      poseidon: poseidon,
      assets: [index_1.FEE_ASSET, index_1.MINT],
      amounts: [new anchor.BN(depositFeeAmount), new anchor.BN(depositAmount)],
      keypair,
    });
    let outputUtxos = [deposit_utxo1];
    console.log(
      "outputUtxos[0].assetsCircuit[1]: ",
      outputUtxos[0].assetsCircuit[1]
    );
    yield tx.prepareTransactionFull({
      inputUtxos: [],
      outputUtxos,
      action: "DEPOSIT",
      assetPubkeys: [new anchor.BN(0), outputUtxos[0].assetsCircuit[1]],
      relayerFee: 0,
      sender: web3_js_1.SystemProgram.programId,
      mintPubkey: (0, index_1.hashAndTruncateToCircuit)(index_1.MINT.toBytes()),
      merkleTreeAssetPubkey: index_1.REGISTERED_POOL_PDA_SPL_TOKEN,
      config: { in: 2, out: 2 },
    });
    // successful proofgen
    yield tx.getProof();
    // unsuccessful proofgen
    tx.inIndices[0][1][1] = "1";
    // TODO: investigate why this does not kill the proof
    tx.inIndices[0][1][0] = "1";
    try {
      (0, chai_1.expect)(yield tx.getProof()).to.Throw();
      // console.log(tx.input.inIndices[0])
      // console.log(tx.input.inIndices[1])
    } catch (error) {
      chai_1.assert.isTrue(error.toString().includes("CheckIndices_3 line:"));
    }
  });
}
exports.functionalCircuitTest = functionalCircuitTest;
