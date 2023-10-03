const bench = require("micro-bmark");
const { compare, run } = bench;
const snarkjs = require("snarkjs");
const buildPoseidonOpt = require("circomlibjs").buildPoseidonOpt;

import { genRandomSalt as generateRandomFieldElement } from "maci-crypto";
import { execSync } from "child_process";
import { resolve } from "path";
import { writeFileSync, existsSync, promises } from "fs";
import { Keypair as SolanaKeypair, PublicKey } from "@solana/web3.js";
import { BN } from "@coral-xyz/anchor";

import {
  Account,
  Utxo,
  FEE_ASSET,
  Provider as LightProvider,
  MINT,
  Transaction,
  TransactionParameters,
  Action,
  IDL_VERIFIER_PROGRAM_ZERO,
  IDL_VERIFIER_PROGRAM_TWO,
  STANDARD_SHIELDED_PRIVATE_KEY,
  STANDARD_SHIELDED_PUBLIC_KEY,
} from "@lightprotocol/zk.js";

import { bs58 } from "@coral-xyz/anchor/dist/cjs/utils/bytes";
import { MerkleTree } from "../../../circuit-lib.js/lib/index.js";
import { stringifyBigInts } from "maci-crypto";

// This will extract the type of the constructor parameters of Transaction CLass
type TransactionInputs = ConstructorParameters<typeof Transaction>[0];

export type circuitParameterVariants = {
  merkleTreeHeightVariants: number[];
  nInputUtxosVariants: number[];
  nOutputUtxosVariants: number[];
  outputPath: string;
};

type CircuitModel = {
  name: string;
  merkleTreeHeight: number;
  merkleTree?: MerkleTree;
  nInputUtxos: number;
  nOutputUtxos: number;
};

// skip unwanted logs
console.log = (function (originalLog) {
  const skipPatterns = [
    "Provider is not defined.",
    "Unable to fetch rootIndex. Setting root index to 0 as a default value.",
    // Add more patterns if needed
  ];

  return function (...args) {
    const message = args.join(" ");

    if (!skipPatterns.some((pattern) => message.includes(pattern))) {
      originalLog.apply(console, args);
    }
  };
})(console.log);

export class BenchmarkLightCircuit {
  app: boolean;
  circuitType: string;
  overwrite: boolean;
  circuitParameterVariants: circuitParameterVariants;
  circuitModels: Array<CircuitModel>;
  merkleTreeElements: Array<string>;
  poseidon: any;

  constructor(
    circuitParameterVariants: circuitParameterVariants,
    app = false,
    overwrite = true,
  ) {
    this.app = app;
    this.circuitType = app ? "app" : "masp";
    this.overwrite = overwrite;
    this.circuitParameterVariants = circuitParameterVariants;
    this.circuitModels = [];
    this.merkleTreeElements = Array.from({ length: 2 ** 13 }, () =>
      generateRandomFieldElement().toString(),
    );
  }

  private async ensureDirectoryExists(dirPath: string) {
    try {
      await promises.access(dirPath);
    } catch (error) {
      if (error.code === "ENOENT") {
        // If directory doesn't exist, create it
        await promises.mkdir(dirPath, { recursive: true });
      } else {
        // Rethrow the error if it's not the expected "directory doesn't exist" error
        throw error;
      }
    }
  }

  private combineArrays(...arrays) {
    if (arrays.length === 0) return [[]];
    const [first, ...rest] = arrays;
    const suffixes = this.combineArrays(...rest);
    return first.flatMap((item) => suffixes.map((suffix) => [item, ...suffix]));
  }

  private async generateCircomMainFile(
    merkleTreeHeight: number,
    nInputUtxos: number,
    nOutputUtxos: number,
  ) {
    const circomTemplate =
      `
    pragma circom 2.0.0;
    include "../../../src/light/transaction_${this.circuitType}.circom";

    // 2 in 2 out 3 assets (min to do a swap)
    component main {
        public [
            root,
            inputNullifier,
            outputCommitment,
            publicAmountSpl,
            txIntegrityHash,
            publicAmountSol,
            publicMintPubkey` +
      `${this.app ? "," : ""}
            ${this.app ? "publicAppVerifier," : ""}
            ${this.app ? "transactionHash" : ""}
            `.trimEnd() +
      `\n        ]
    } = TransactionAccount(
        ${merkleTreeHeight},
        ${nInputUtxos},
        ${nOutputUtxos},
        184598798020101492503359154328231866914977581098629757339001774613643340069,
        0,
        1,
        3,
        2,
        2
    );
        `;
    const circuitName = `transaction${nInputUtxos}in${nOutputUtxos}out${
      this.app ? "App" : "Masp"
    }MTH${merkleTreeHeight}`;

    this.circuitModels.push({
      name: circuitName,
      merkleTreeHeight,
      nInputUtxos,
      nOutputUtxos,
    });
    const finalPath =
      this.circuitParameterVariants.outputPath + "/" + circuitName + ".circom";

    // Check if file already exists
    if (!this.overwrite && existsSync(finalPath)) {
      return;
    }

    // Write the generated content to the outputPath
    writeFileSync(finalPath, circomTemplate.trim());
  }

  async generateCircomMainFiles() {
    const {
      merkleTreeHeightVariants,
      nInputUtxosVariants,
      nOutputUtxosVariants,
    } = this.circuitParameterVariants;

    await this.ensureDirectoryExists(this.circuitParameterVariants.outputPath);

    const parameterVariants = this.combineArrays(
      merkleTreeHeightVariants,
      nInputUtxosVariants,
      nOutputUtxosVariants,
    );

    for (const variant of parameterVariants) {
      this.generateCircomMainFile(variant[0], variant[1], variant[2]);
    }
  }

  private addPrivateKey(tx: Transaction, account: Account, seed32: string) {
    const account_privateKey = Account.generateShieldedPrivateKey(
      seed32,
      tx.params.poseidon,
    );
    const account_publicKey = Account.generateShieldedPublicKey(
      account_privateKey,
      tx.params.poseidon,
    );

    tx.proofInput["inPrivateKey"] = tx.params.inputUtxos.map((utxo: Utxo) => {
      if (
        utxo.publicKey.eq(account.pubkey) &&
        account.pubkey.eq(account_publicKey)
      ) {
        return account_privateKey;
      }
      if (STANDARD_SHIELDED_PUBLIC_KEY.eq(utxo.publicKey)) {
        return STANDARD_SHIELDED_PRIVATE_KEY;
      }
    });
  }

  // for now the benchmarks are solely dependant on the Merkle Tree Height
  async generateProofInputs(merkleTree: MerkleTree) {
    const lightProvider = await LightProvider.loadMock();

    const seed32 = bs58.encode(new Uint8Array(32).fill(1));
    const account = new Account({ poseidon: this.poseidon, seed: seed32 });
    await account.getEddsaPublicKey();

    const depositAmount = 20_000;
    const depositFeeAmount = 10_000;

    const deposit_utxo1 = new Utxo({
      poseidon: this.poseidon,
      assets: [FEE_ASSET, MINT],
      amounts: [new BN(depositFeeAmount), new BN(depositAmount)],
      publicKey: account.pubkey,
      assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
      verifierProgramLookupTable:
        lightProvider.lookUpTables.verifierProgramLookupTable,
    });

    const mockPubkey: PublicKey = SolanaKeypair.generate().publicKey;

    const txParams = new TransactionParameters({
      outputUtxos: [deposit_utxo1],
      eventMerkleTreePubkey: mockPubkey,
      transactionMerkleTreePubkey: mockPubkey,
      senderSpl: mockPubkey,
      senderSol: lightProvider.wallet.publicKey,
      action: Action.SHIELD,
      poseidon: this.poseidon,
      verifierIdl: this.app
        ? IDL_VERIFIER_PROGRAM_TWO
        : IDL_VERIFIER_PROGRAM_ZERO,
      account,
    });

    lightProvider.solMerkleTree!.merkleTree = merkleTree;
    // lightProvider.solMerkleTree!.merkleTree.insert(deposit_utxo1.getCommitment(this.poseidon))

    const txInputs: TransactionInputs = {
      ...(await lightProvider.getRootIndex()),
      solMerkleTree: lightProvider.solMerkleTree!,
      params: txParams,
    };

    if (this.app) {
      txInputs.appParams = {
        mock: "123",
        verifierIdl: IDL_VERIFIER_PROGRAM_ZERO,
      };
    }

    const tx = new Transaction(txInputs);
    await tx.compile(lightProvider.poseidon, account);

    this.addPrivateKey(tx, account, seed32);
    if (this.app) delete tx.proofInput.inPublicKey;

    return tx.proofInput;
  }

  async bench_transaction_account(circuitModel: CircuitModel) {
    const first_path = `./artifacts/${circuitModel.name}/${circuitModel.name}`;
    const wasm_path = first_path + ".wasm";
    const zkey_path = first_path + ".zkey";

    const proofInputs = await this.generateProofInputs(
      circuitModel.merkleTree!,
    );
    const res = await snarkjs.groth16.fullProve(
      stringifyBigInts(proofInputs),
      wasm_path,
      zkey_path,
    );

    return res;
  }

  initializeMerkleTree(circuitModel: CircuitModel) {
    circuitModel.merkleTree = new MerkleTree(
      circuitModel.merkleTreeHeight,
      this.poseidon,
      this.merkleTreeElements,
    );
  }

  async benchmark(iterations: number) {
    this.poseidon = await buildPoseidonOpt();
    // create circuit main templates based on the input variants
    await this.generateCircomMainFiles();
    const buildScriptPath = resolve(
      __dirname,
      "../../scripts/buildAllBenchCircuits.sh",
    );

    try {
      // Execute the script that builds all the Benchmark Circuit Template Variants
      execSync(buildScriptPath, { stdio: "inherit" });
    } catch (error) {
      console.error("Error executing buildAllBenchCircuits script:", error);
    }

    const callbacks: Record<string, () => Promise<void>> = {};
    for (const circuitModel of this.circuitModels) {
      this.initializeMerkleTree(circuitModel);
      callbacks[`${circuitModel.name}`] = async () =>
        await this.bench_transaction_account(circuitModel);
    }

    console.log("Iterations: ", iterations);
    return run(async () => {
      await compare(
        `\n\x1b[35mBenchmarking transaction_${this.circuitType} circuit\x1b[0m`,
        iterations,
        callbacks,
      );
      bench.utils.logMem(); // Log current RAM
      (globalThis as any).curve_bn128.terminate();
    });
  }
}
