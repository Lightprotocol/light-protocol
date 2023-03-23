import { assert, expect } from "chai";
const chai = require("chai");
const chaiAsPromised = require("chai-as-promised");
const should = chai.should();
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
} from "../src";
process.env.ANCHOR_PROVIDER_URL = "http://127.0.0.1:8899";
process.env.ANCHOR_WALLET = process.env.HOME + "/.config/solana/id.json";

const verifiers = [
  // { verifier: new VerifierZero(), isApp: false },
  // { verifier: new VerifierOne(), isApp: false },
  { verifier: new VerifierTwo(), isApp: true },
];

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
