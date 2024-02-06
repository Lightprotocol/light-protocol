import { it, beforeAll, expect } from "vitest";

import {
  functionalCircuitTest,
  IDL_LIGHT_PSP2IN2OUT,
  IDL_LIGHT_PSP10IN2OUT,
  IDL_LIGHT_PSP4IN4OUT_APP_STORAGE,
  IDL_PUBLIC_LIGHT_PSP2IN2OUT,
  IDL_PUBLIC_LIGHT_PSP10IN2OUT,
  getVerifierProgramId,
  FEE_ASSET,
  Account,
  Provider as LightProvider,
  MINT,
  createTransaction,
  TransactionInput,
  getSystemProof,
  createSystemProofInputs,
  hashAndTruncateToCircuit,
  BN_0,
  getTransactionHash,
  createOutUtxo,
  outUtxoToUtxo,
  OutUtxo,
  Utxo,
  BN_1,
  BN_2,
  createProgramOutUtxo,
  PlaceHolderTData,
  ProgramUtxo,
  ProgramOutUtxo,
  programOutUtxoToProgramUtxo,
} from "../src";
import { WasmFactory } from "@lightprotocol/account.rs";
import { BN } from "@coral-xyz/anchor";
import { Keypair as SolanaKeypair } from "@solana/web3.js";
import { Idl } from "@coral-xyz/anchor";
import { bs58 } from "@coral-xyz/anchor/dist/cjs/utils/bytes";
import { MerkleTree } from "@lightprotocol/circuit-lib.js";

process.env.ANCHOR_PROVIDER_URL = "http://127.0.0.1:8899";
process.env.ANCHOR_WALLET = process.env.HOME + "/.config/solana/id.json";
const wasm_tester = require("circom_tester").wasm;

function getSignalByName(
  circuit: any,
  witness: any,
  signalName: string,
): string {
  const signal = `main.${signalName}`;
  return witness[circuit.symbols[signal].varIdx].toString();
}
async function getWasmTester(verifierIdl: Idl) {
  const basePath = "../circuit-lib/circuit-lib.circom/src/transaction/";
  const circuitPathMap = new Map<string, string>([
    ["light_public_psp2in2out", "publicProgramTransaction2In2OutMain.circom"],
    ["light_public_psp10in2out", "publicTransaction10In2OutMain.circom"],
  ]);
  const path = basePath + circuitPathMap.get(verifierIdl.name);

  console.time("wasm_tester");
  const circuit = await wasm_tester(path, {
    include:
      "../circuit-lib/circuit-lib.circom/node_modules/circomlib/circuits/",
  });
  console.timeEnd("wasm_tester");
  return circuit;
}
const getTestProver = async (
  verifierIdl: Idl,
  firstPath: string,
  circuitName: string,
  wasmTester: any,
) => {
  wasmTester = await wasmTester;
  return new TestProver(wasmTester, verifierIdl, circuitName);
};

// TODO: add specific getTestProver function which does the test with hardcoded assert values

class TestProver {
  circuit: any;
  proofInputs: any;
  idl: Idl;
  circuitName: string;
  constructor(circuit: any, idl: Idl, circuitName: string) {
    this.circuit = circuit;
    this.idl = idl;
    if (!circuitName) {
      const ZKAccountNames = this.idl.accounts
        ?.filter((account) =>
          /zK.*(?:PublicInputs|ProofInputs)|zk.*(?:PublicInputs|ProofInputs)/.test(
            account.name,
          ),
        )
        .map((account) => account.name);

      // Extract the circuit names and store them in a Set to get unique names
      const circuitNameRegex =
        /zK(.*?)ProofInputs|zK(.*?)PublicInputs|zk(.*?)ProofInputs|zk(.*?)PublicInputs/;
      const uniqueCircuitNames = new Set<string>();

      ZKAccountNames?.forEach((name) => {
        const match = name.match(circuitNameRegex);
        if (match) {
          uniqueCircuitNames.add(match[1] || match[2] || match[3] || match[4]);
        }
      });

      this.circuitName = Array.from(uniqueCircuitNames)[0] as string;
    } else {
      this.circuitName = circuitName;
    }
    if (!this.circuitName) throw new Error("Circuit name is undefined");
  }

  public addProofInputs(proofInputs: any) {
    const circuitIdlObject = this.idl.accounts!.find(
      (account) =>
        account.name.toUpperCase() ===
        `zK${this.circuitName}ProofInputs`.toUpperCase(),
    );

    if (!circuitIdlObject) {
      throw new Error(
        `${`zK${this.circuitName}ProofInputs`} does not exist in anchor idl`,
      );
    }

    const fieldNames = circuitIdlObject.type.fields.map(
      (field: { name: string }) => field.name,
    );
    const inputKeys: string[] = [];

    fieldNames.forEach((fieldName: string) => {
      inputKeys.push(fieldName);
    });

    const inputsObject: { [key: string]: any } = {};
    const missingInputs: string[] = [];
    inputKeys.forEach((key) => {
      inputsObject[key] = proofInputs[key];
      if (!inputsObject[key]) missingInputs.push(key);
    });
    if (missingInputs.length > 0) {
      let errorString = "";
      for (const key of missingInputs) {
        errorString += `Missing input: ${key.toString()} \n`;
      }
      errorString += `Circuit: ${this.circuitName}`;
      throw new Error(errorString);
    }
    this.proofInputs = inputsObject;
  }

  public async fullProveAndParse() {
    const witness = await this.circuit.calculateWitness(this.proofInputs);
    await this.circuit.checkConstraints(witness);
    await this.circuit.loadSymbols();
  }

  getSignalByName(signalName: string): string {
    return getSignalByName(this.circuit, this.proofInputs, signalName);
  }
}

const verifiers = [
  { verifierIdl: IDL_LIGHT_PSP2IN2OUT, isApp: false },
  { verifierIdl: IDL_LIGHT_PSP10IN2OUT, isApp: false },
  { verifierIdl: IDL_LIGHT_PSP4IN4OUT_APP_STORAGE, isApp: true },
];

const publicVerifiers = [
  { verifierIdl: IDL_PUBLIC_LIGHT_PSP2IN2OUT, isApp: true },
  { verifierIdl: IDL_PUBLIC_LIGHT_PSP10IN2OUT, isApp: false },
];

describe("Verifier tests", () => {
  it("Test functional private circuits", async () => {
    for (const verifier in verifiers) {
      console.time("private circuits" + verifiers[verifier].verifierIdl.name);
      for (let i = 0; i < 1; i++) {
        await functionalCircuitTest(
          verifiers[verifier].isApp,
          verifiers[verifier].verifierIdl,
          getVerifierProgramId(verifiers[verifier].verifierIdl),
        );
      }
      console.timeEnd(
        "private circuits" + verifiers[verifier].verifierIdl.name,
      );
    }
  });

  it("Test functional public circuits", async () => {
    console.time("public circuits");
    for (const verifier in publicVerifiers) {
      console.time(
        "public circuits" + publicVerifiers[verifier].verifierIdl.name,
      );
      for (let i = 0; i < 1; i++) {
        await functionalCircuitTest(
          publicVerifiers[verifier].isApp,
          publicVerifiers[verifier].verifierIdl,
          getVerifierProgramId(publicVerifiers[verifier].verifierIdl),
        );
      }
      console.timeEnd(
        "public circuits" + publicVerifiers[verifier].verifierIdl.name,
      );
    }
  });

  let lightWasm,
    lightProvider,
    mockPubkey,
    inputUtxo: OutUtxo | Utxo,
    outputUtxo1,
    merkleTree: MerkleTree,
    outputUtxo2: OutUtxo | Utxo,
    inputUtxo2: OutUtxo | Utxo,
    seed32,
    account,
    shieldAmount,
    shieldFeeAmount,
    inputProgramUtxo:
      | ProgramUtxo<PlaceHolderTData>
      | ProgramOutUtxo<PlaceHolderTData>,
    rpcFee,
    wasmTester2in2out,
    wasmTester10in2out,
    plainInputUtxo: Utxo | OutUtxo;
  beforeAll(async () => {
    lightProvider = await LightProvider.loadMock();
    mockPubkey = SolanaKeypair.generate().publicKey;

    lightWasm = await WasmFactory.getInstance();
    wasmTester2in2out = getWasmTester(IDL_PUBLIC_LIGHT_PSP2IN2OUT);
    wasmTester10in2out = getWasmTester(IDL_PUBLIC_LIGHT_PSP10IN2OUT);
    seed32 = bs58.encode(new Uint8Array(32).fill(1));
    account = Account.createFromSeed(lightWasm, seed32);
    shieldAmount = 20_000;
    shieldFeeAmount = 10_000;
    rpcFee = new BN(5000);
    plainInputUtxo = createOutUtxo({
      lightWasm,
      assets: [FEE_ASSET, MINT],
      amounts: [new BN(shieldFeeAmount), new BN(shieldAmount)],
      owner: account.keypair.publicKey,
    });

    inputUtxo = createOutUtxo({
      lightWasm,
      assets: [FEE_ASSET, MINT],
      amounts: [new BN(shieldFeeAmount), new BN(shieldAmount)],
      owner: account.keypair.publicKey,
      metaHash: BN_1,
      address: BN_2,
    });

    inputUtxo2 = createOutUtxo({
      lightWasm,
      assets: [FEE_ASSET],
      amounts: [BN_0],
      owner: account.keypair.publicKey,
      metaHash: new BN(3),
      address: new BN(4),
    });

    inputProgramUtxo = createProgramOutUtxo({
      lightWasm,
      assets: [FEE_ASSET],
      amounts: [BN_0],
      owner: getVerifierProgramId(IDL_PUBLIC_LIGHT_PSP2IN2OUT),
      metaHash: new BN(6),
      address: new BN(7),
      data: { rnd: 1 },
      ownerIdl: IDL_PUBLIC_LIGHT_PSP2IN2OUT,
      /// Allows for custom hashing schema
      /// One could instead also use the default hashing util: createDataHashWithDefaultHashingSchema()
      dataHash: BN_1,
      type: "",
    });

    merkleTree = new MerkleTree(22, lightWasm, [
      inputUtxo.hash.toString(),
      inputUtxo2.hash.toString(),
      inputProgramUtxo.hash.toString(),
      plainInputUtxo.hash.toString(),
    ]);
    inputUtxo = outUtxoToUtxo(
      inputUtxo,
      merkleTree.path(0).pathElements,
      0,
      lightWasm,
      account,
    );
    inputUtxo2 = outUtxoToUtxo(
      inputUtxo2,
      merkleTree.path(1).pathElements,
      1,
      lightWasm,
      account,
    );

    inputProgramUtxo = programOutUtxoToProgramUtxo(
      inputProgramUtxo,
      merkleTree.path(2).pathElements,
      2,
      lightWasm,
      account,
    );

    plainInputUtxo = outUtxoToUtxo(
      plainInputUtxo,
      merkleTree.path(3).pathElements,
      3,
      lightWasm,
      account,
    );

    outputUtxo1 = createOutUtxo({
      lightWasm,
      assets: [FEE_ASSET, MINT],
      amounts: [
        new BN(shieldFeeAmount / 2).sub(rpcFee),
        new BN(shieldAmount / 2),
      ],
      owner: account.keypair.publicKey,
      metaHash: BN_1,
      address: BN_2,
    });

    outputUtxo2 = createOutUtxo({
      lightWasm,
      assets: [FEE_ASSET, MINT],
      amounts: [new BN(shieldFeeAmount / 2), new BN(shieldAmount / 2)],
      owner: account.keypair.publicKey,
      metaHash: new BN(3),
      address: new BN(4),
    });
  });

  it("Test utxos with meta hash and address should succeed", async () => {
    const verifierIdl = IDL_PUBLIC_LIGHT_PSP2IN2OUT;
    const localInputUtxo = inputUtxo;
    const localInputUtxo2 = localInputUtxo;

    const txInput: TransactionInput = {
      inputUtxos: [localInputUtxo2 as Utxo, inputUtxo2 as Utxo],
      outputUtxos: [outputUtxo1, outputUtxo2],
      merkleTreeSetPubkey: mockPubkey,
      lightWasm,
      account,
      rpcFee,
      systemPspId: getVerifierProgramId(verifierIdl),
      rpcPublicKey: lightProvider.rpc.accounts.rpcPubkey,
      pspId: getVerifierProgramId(verifierIdl),
    };

    const transaction = await createTransaction(txInput);
    let systemProofInputs = createSystemProofInputs({
      transaction: transaction,
      lightWasm,
      account,
      root: merkleTree.root(),
    });

    const publicTransactionHash = getTransactionHash(
      transaction.private.inputUtxos,
      transaction.private.outputUtxos,
      BN_0, // is not checked in circuit
      lightWasm,
    );

    systemProofInputs = {
      ...systemProofInputs,
      publicProgramId: hashAndTruncateToCircuit(mockPubkey.toBytes()),
      publicTransactionHash,
      privatePublicDataHash: "0",
      publicDataHash: "0",
    } as any;

    // throws if proof generation fails
    await getSystemProof({
      account,
      inputUtxos: transaction.private.inputUtxos,
      verifierIdl,
      systemProofInputs,
    });
  });

  it("Test add publicNewAddress should succeed", async () => {
    const outputUtxo2 = createOutUtxo({
      lightWasm,
      assets: [FEE_ASSET, MINT],
      amounts: [new BN(shieldFeeAmount / 2), new BN(shieldAmount / 2)],
      owner: account.keypair.publicKey,
      address: new BN(4),
    });
    const verifierIdl = IDL_PUBLIC_LIGHT_PSP2IN2OUT;
    const localInputUtxo = { ...inputUtxo };
    const localInputUtxo2 = localInputUtxo;
    const txInput: TransactionInput = {
      inputUtxos: [localInputUtxo2 as Utxo],
      outputUtxos: [outputUtxo1, outputUtxo2],
      merkleTreeSetPubkey: mockPubkey,
      lightWasm,
      account,
      rpcFee,
      systemPspId: getVerifierProgramId(verifierIdl),
      rpcPublicKey: lightProvider.rpc.accounts.rpcPubkey,
      pspId: getVerifierProgramId(verifierIdl),
    };

    const transaction = await createTransaction(txInput);
    let systemProofInputs = createSystemProofInputs({
      transaction: transaction,
      lightWasm,
      account,
      root: merkleTree.root(),
    });

    const publicTransactionHash = getTransactionHash(
      transaction.private.inputUtxos,
      transaction.private.outputUtxos,
      BN_0, // is not checked in circuit
      lightWasm,
    );
    systemProofInputs = {
      ...systemProofInputs,
      publicProgramId: hashAndTruncateToCircuit(mockPubkey.toBytes()),
      publicTransactionHash: "0",
      privatePublicDataHash: "0",
      publicDataHash: "0",
    } as any;
    systemProofInputs.isNewAddress[1] = BN_1.toString();
    systemProofInputs.publicNewAddress[1] = new BN(4).toString();

    // we rely on the fact that the function throws an error if proof generation failed
    await expect(
        getSystemProof({
        account,
        inputUtxos: transaction.private.inputUtxos,
        verifierIdl,
        systemProofInputs,
        getProver: getTestProver,
        wasmTester: wasmTester2in2out,
      }))
        .rejects.toThrow("PROOF_GENERATION_FAILED: TypeError: Cannot read properties of undefined (reading 'parsedProof')");
  });

  // public input optionality is checked correctly
  it("Test pass for unused utxo slots zero as publicInUtxoHash, publicInUtxoDataHash, publicOutUtxoDataHash", async () => {
    const outputUtxo2 = createOutUtxo({
      lightWasm,
      assets: [FEE_ASSET, MINT],
      amounts: [new BN(shieldFeeAmount).sub(rpcFee), new BN(shieldAmount)],
      owner: account.keypair.publicKey,
      metaHash: inputUtxo.metaHash,
      address: inputUtxo.address,
    });
    const verifierIdl = IDL_PUBLIC_LIGHT_PSP2IN2OUT;
    const localInputUtxo = { ...inputUtxo };
    const localInputUtxo2 = localInputUtxo;
    const txInput: TransactionInput = {
      inputUtxos: [localInputUtxo2 as Utxo],
      outputUtxos: [outputUtxo2],
      merkleTreeSetPubkey: mockPubkey,
      lightWasm,
      account,
      rpcFee,
      systemPspId: getVerifierProgramId(verifierIdl),
      rpcPublicKey: lightProvider.rpc.accounts.rpcPubkey,
      pspId: getVerifierProgramId(verifierIdl),
    };

    const transaction = await createTransaction(txInput);
    let systemProofInputs = createSystemProofInputs({
      transaction: transaction,
      lightWasm,
      account,
      root: merkleTree.root(),
    });

    const publicTransactionHash = getTransactionHash(
      transaction.private.inputUtxos,
      transaction.private.outputUtxos,
      BN_0, // is not checked in circuit
      lightWasm,
    );
    systemProofInputs = {
      ...systemProofInputs,
      publicProgramId: hashAndTruncateToCircuit(mockPubkey.toBytes()),
      publicTransactionHash,
      privatePublicDataHash: "0",
      publicDataHash: "0",
    } as any;

    // adjustment for publicInUtxoHash
    systemProofInputs.publicInUtxoHash[1] = BN_0.toString();
    systemProofInputs.publicInUtxoDataHash[1] = BN_0.toString();
    systemProofInputs.publicOutUtxoHash[1] = BN_0;

    try {
      await getSystemProof({
        account,
        inputUtxos: transaction.private.inputUtxos,
        verifierIdl,
        systemProofInputs,
        getProver: getTestProver,
        wasmTester: wasmTester2in2out,
      });
    } catch (e: any) {
      console.log("e", e);
      if (
        !e
          .toString()
          .includes(
            "TypeError: Cannot read properties of undefined (reading 'parsedProof')",
          )
      ) {
        throw new Error("Proof generation failed");
      }
    }
    const publicOutUtxoHash = systemProofInputs.publicOutUtxoHash[0];
    systemProofInputs.publicOutUtxoHash[0] = BN_0;

    // need to be very careful with the expected error here because this will always return a type error when parsing after successful witness generation fails
    await expect(
      getSystemProof({
        account,
        inputUtxos: transaction.private.inputUtxos,
        verifierIdl,
        systemProofInputs,
        getProver: getTestProver,
        wasmTester: wasmTester2in2out,
      })).rejects.toThrow("PROOF_GENERATION_FAILED: Error: Error: Assert Failed.");

    systemProofInputs.publicOutUtxoHash[0] = publicOutUtxoHash;
    systemProofInputs.publicInUtxoHash[0] = BN_0.toString();

    // need to be very careful with the expected error here because this will always return a type error when parsing after successful witness generation fails
    await expect(
      getSystemProof({
        account,
        inputUtxos: transaction.private.inputUtxos,
        verifierIdl,
        systemProofInputs,
        getProver: getTestProver,
        wasmTester: wasmTester2in2out,
      })).rejects.toThrow("PROOF_GENERATION_FAILED: Error: Error: Assert Failed.");
  });

  it("Test spend utxo which is not in Merkle tree should not succeed", async () => {
    const outputUtxo2 = createOutUtxo({
      lightWasm,
      assets: [FEE_ASSET, MINT],
      amounts: [new BN(shieldFeeAmount).sub(rpcFee), new BN(shieldAmount)],
      owner: account.keypair.publicKey,
      metaHash: inputUtxo.metaHash,
      address: inputUtxo.address,
    });
    const verifierIdl = IDL_PUBLIC_LIGHT_PSP2IN2OUT;
    const localInputUtxo = { ...inputUtxo };
    const localInputUtxo2 = localInputUtxo;
    localInputUtxo.blinding = BN_1;
    const txInput: TransactionInput = {
      inputUtxos: [localInputUtxo2 as Utxo],
      outputUtxos: [outputUtxo2],
      merkleTreeSetPubkey: mockPubkey,
      lightWasm,
      account,
      rpcFee,
      systemPspId: getVerifierProgramId(verifierIdl),
      rpcPublicKey: lightProvider.rpc.accounts.rpcPubkey,
      pspId: getVerifierProgramId(verifierIdl),
    };

    const transaction = await createTransaction(txInput);
    let systemProofInputs = createSystemProofInputs({
      transaction: transaction,
      lightWasm,
      account,
      root: merkleTree.root(),
    });

    const publicTransactionHash = getTransactionHash(
      transaction.private.inputUtxos,
      transaction.private.outputUtxos,
      BN_0, // is not checked in circuit
      lightWasm,
    );
    systemProofInputs = {
      ...systemProofInputs,
      publicProgramId: hashAndTruncateToCircuit(mockPubkey.toBytes()),
      publicTransactionHash,
      privatePublicDataHash: "0",
      publicDataHash: "0",
    } as any;

    // adjustment for publicInUtxoHash
    systemProofInputs.publicInUtxoHash[1] = BN_0.toString();
    systemProofInputs.publicInUtxoDataHash[1] = BN_0.toString();
    systemProofInputs.publicOutUtxoHash[1] = BN_0;
    await expect(
      getSystemProof({
        account,
        inputUtxos: transaction.private.inputUtxos,
        verifierIdl,
        systemProofInputs,
        getProver: getTestProver,
        wasmTester: wasmTester2in2out,
      })).rejects.toThrow("PROOF_GENERATION_FAILED: Error: Error: Assert Failed.");
  });

  it("Test address is persistent", async () => {
    const outputUtxo2 = createOutUtxo({
      lightWasm,
      assets: [FEE_ASSET, MINT],
      amounts: [new BN(shieldFeeAmount).sub(rpcFee), new BN(shieldAmount)],
      owner: account.keypair.publicKey,
      metaHash: inputUtxo.metaHash,
    });
    const verifierIdl = IDL_PUBLIC_LIGHT_PSP2IN2OUT;
    const localInputUtxo = { ...inputUtxo };
    const localInputUtxo2 = localInputUtxo;

    const txInput: TransactionInput = {
      inputUtxos: [localInputUtxo2 as Utxo],
      outputUtxos: [outputUtxo2],
      merkleTreeSetPubkey: mockPubkey,
      lightWasm,
      account,
      rpcFee,
      systemPspId: getVerifierProgramId(verifierIdl),
      rpcPublicKey: lightProvider.rpc.accounts.rpcPubkey,
      pspId: getVerifierProgramId(verifierIdl),
    };

    const transaction = await createTransaction(txInput);
    let systemProofInputs = createSystemProofInputs({
      transaction: transaction,
      lightWasm,
      account,
      root: merkleTree.root(),
    });

    const publicTransactionHash = getTransactionHash(
      transaction.private.inputUtxos,
      transaction.private.outputUtxos,
      BN_0, // is not checked in circuit
      lightWasm,
    );
    systemProofInputs = {
      ...systemProofInputs,
      publicProgramId: hashAndTruncateToCircuit(mockPubkey.toBytes()),
      publicTransactionHash,
      privatePublicDataHash: "0",
      publicDataHash: "0",
    } as any;

    // need to set this manually because it is automatically taken from the outputUtxo address which we set zero on purpose
    systemProofInputs.isAddressUtxo[0] = BN_1;
    await expect(
      getSystemProof({
        account,
        inputUtxos: transaction.private.inputUtxos,
        verifierIdl,
        systemProofInputs,
        getProver: getTestProver,
        wasmTester: wasmTester2in2out,
      })).rejects.toThrow("PROOF_GENERATION_FAILED: Error: Error: Assert Failed.");
  });

  it("Test metaHash is persistent", async () => {
    const outputUtxo2 = createOutUtxo({
      lightWasm,
      assets: [FEE_ASSET, MINT],
      amounts: [new BN(shieldFeeAmount).sub(rpcFee), new BN(shieldAmount)],
      owner: account.keypair.publicKey,
      address: inputUtxo.address,
    });
    const verifierIdl = IDL_PUBLIC_LIGHT_PSP2IN2OUT;
    const localInputUtxo = { ...inputUtxo };
    const localInputUtxo2 = localInputUtxo;

    const txInput: TransactionInput = {
      inputUtxos: [localInputUtxo2 as Utxo],
      outputUtxos: [outputUtxo2],
      merkleTreeSetPubkey: mockPubkey,
      lightWasm,
      account,
      rpcFee,
      systemPspId: getVerifierProgramId(verifierIdl),
      rpcPublicKey: lightProvider.rpc.accounts.rpcPubkey,
      pspId: getVerifierProgramId(verifierIdl),
    };

    const transaction = await createTransaction(txInput);
    let systemProofInputs = createSystemProofInputs({
      transaction: transaction,
      lightWasm,
      account,
      root: merkleTree.root(),
    });

    const publicTransactionHash = getTransactionHash(
      transaction.private.inputUtxos,
      transaction.private.outputUtxos,
      BN_0, // is not checked in circuit
      lightWasm,
    );
    systemProofInputs = {
      ...systemProofInputs,
      publicProgramId: hashAndTruncateToCircuit(mockPubkey.toBytes()),
      publicTransactionHash,
      privatePublicDataHash: "0",
      publicDataHash: "0",
    } as any;

    // need to set this manually because it is automatically taken from the outputUtxo address which we set zero on purpose
    systemProofInputs.isMetaHashUtxo[0] = BN_1;
    await expect(
      getSystemProof({
        account,
        inputUtxos: transaction.private.inputUtxos,
        verifierIdl,
        systemProofInputs,
        getProver: getTestProver,
        wasmTester: wasmTester2in2out,
      })).rejects.toThrow("PROOF_GENERATION_FAILED: Error: Error: Assert Failed.");
  });

  it("Test programUtxo works & publicUtxoDataHash needs to be set if utxo has dataHash, transactionHash is optional", async () => {
    const outputUtxo2 = createOutUtxo({
      lightWasm,
      assets: [FEE_ASSET],
      amounts: [BN_0],
      owner: account.keypair.publicKey,
      metaHash: inputProgramUtxo.metaHash,
      address: inputProgramUtxo.address,
    });
    const verifierIdl = IDL_PUBLIC_LIGHT_PSP2IN2OUT;
    const localInputUtxo = inputProgramUtxo;
    const localInputUtxo2 = localInputUtxo;

    const txInput: TransactionInput = {
      inputUtxos: [localInputUtxo2 as ProgramUtxo<PlaceHolderTData>],
      outputUtxos: [outputUtxo2],
      merkleTreeSetPubkey: mockPubkey,
      lightWasm,
      account,
      rpcFee,
      systemPspId: getVerifierProgramId(verifierIdl),
      rpcPublicKey: lightProvider.rpc.accounts.rpcPubkey,
      pspId: getVerifierProgramId(verifierIdl),
    };

    const transaction = await createTransaction(txInput);
    let systemProofInputs = createSystemProofInputs({
      transaction: transaction,
      lightWasm,
      account,
      root: merkleTree.root(),
    });

    const publicTransactionHash = getTransactionHash(
      transaction.private.inputUtxos,
      transaction.private.outputUtxos,
      BN_0, // is not checked in circuit
      lightWasm,
    );

    systemProofInputs = {
      ...systemProofInputs,
      publicProgramId: hashAndTruncateToCircuit(
        getVerifierProgramId(IDL_PUBLIC_LIGHT_PSP2IN2OUT).toBytes(),
      ),
      publicTransactionHash,
      privatePublicDataHash: "0",
      publicDataHash: "0",
    } as any;

    systemProofInputs.isInProgramUtxo[0] = BN_1;
    systemProofInputs.inOwner[0] = hashAndTruncateToCircuit(
      getVerifierProgramId(IDL_PUBLIC_LIGHT_PSP2IN2OUT).toBytes(),
    );

    try {
      await getSystemProof({
        account,
        inputUtxos: transaction.private.inputUtxos,
        verifierIdl,
        systemProofInputs,
        getProver: getTestProver,
        wasmTester: wasmTester2in2out,
      });
    } catch (e: any) {
      console.log("e", e);
      if (
        !e
          .toString()
          .includes(
            "TypeError: Cannot read properties of undefined (reading 'parsedProof')",
          )
      ) {
        throw new Error("Proof generation failed");
      }
    }
    const publicInUtxoDataHash = systemProofInputs.publicInUtxoDataHash[0];
    // adjustment for publicInUtxoHash
    systemProofInputs.publicInUtxoDataHash[0] = BN_0.toString();
    await expect(
      getSystemProof({
        account,
        inputUtxos: transaction.private.inputUtxos,
        verifierIdl,
        systemProofInputs,
        getProver: getTestProver,
        wasmTester: wasmTester2in2out,
      })).rejects.toThrow("PROOF_GENERATION_FAILED: Error: Error: Assert Failed.");

    systemProofInputs.publicInUtxoDataHash[0] = publicInUtxoDataHash;
    console.log(
      "transaction hash is public ",
      systemProofInputs.transactionHashIsPublic,
    );

    // @ts-ignore: its not part of the return type but we set it manually above
    systemProofInputs.publicTransactionHash = BN_1.toString();
    await expect(
      getSystemProof({
        account,
        inputUtxos: transaction.private.inputUtxos,
        verifierIdl,
        systemProofInputs,
        getProver: getTestProver,
        wasmTester: wasmTester2in2out,
      })).rejects.toThrow("PROOF_GENERATION_FAILED: Error: Error: Assert Failed.");
  });

  it("Test 10in2out functional", async () => {
    const outputUtxo2 = createOutUtxo({
      lightWasm,
      assets: [FEE_ASSET, MINT],
      amounts: [new BN(shieldFeeAmount).sub(rpcFee), new BN(shieldAmount)],
      owner: account.keypair.publicKey,
    });
    const verifierIdl = IDL_PUBLIC_LIGHT_PSP10IN2OUT;

    const txInput: TransactionInput = {
      inputUtxos: [plainInputUtxo as Utxo],
      outputUtxos: [outputUtxo2],
      merkleTreeSetPubkey: mockPubkey,
      lightWasm,
      account,
      rpcFee,
      systemPspId: getVerifierProgramId(verifierIdl),
      rpcPublicKey: lightProvider.rpc.accounts.rpcPubkey,
      pspId: getVerifierProgramId(verifierIdl),
    };

    const transaction = await createTransaction(txInput);
    let systemProofInputs = createSystemProofInputs({
      transaction: transaction,
      lightWasm,
      account,
      root: merkleTree.root(),
    });

    const publicTransactionHash = getTransactionHash(
      transaction.private.inputUtxos,
      transaction.private.outputUtxos,
      BN_0, // is not checked in circuit
      lightWasm,
    );
    systemProofInputs = {
      ...systemProofInputs,
      publicProgramId: hashAndTruncateToCircuit(mockPubkey.toBytes()),
      publicTransactionHash,
      privatePublicDataHash: "0",
      publicDataHash: "0",
    } as any;

    await expect(
      getSystemProof({
        account,
        inputUtxos: transaction.private.inputUtxos,
        verifierIdl,
        systemProofInputs,
        getProver: getTestProver,
        wasmTester: wasmTester10in2out,
      })).rejects.toThrow("PROOF_GENERATION_FAILED: TypeError: Cannot read properties of undefined (reading 'parsedProof')");
  });
});
