const anchor = require("@coral-xyz/anchor");
const nacl = require("tweetnacl");
export const createEncryptionKeypair = () => nacl.box.keyPair();
var assert = require("assert");
let circomlibjs = require("circomlibjs");
var ffjavascript = require("ffjavascript");
const { unstringifyBigInts, stringifyBigInts, leInt2Buff, leBuff2int } =
  ffjavascript.utils;
import { close, readFileSync } from "fs";
const snarkjs = require("snarkjs");
const { keccak_256 } = require("@noble/hashes/sha3");

import {
  PublicKey,
  Keypair as SolanaKeypair,
  SystemProgram,
  ComputeBudgetProgram,
  AddressLookupTableAccount,
  TransactionMessage,
  VersionedTransaction,
  TransactionSignature,
  TransactionInstruction,
} from "@solana/web3.js";
import { TOKEN_PROGRAM_ID, getAccount } from "@solana/spl-token";
import { BN, Program, Provider } from "@coral-xyz/anchor";
import { PRE_INSERTED_LEAVES_INDEX, confirmConfig } from "./constants";
import { N_ASSETS, N_ASSET_PUBKEYS, Utxo } from "./utxo";
import { PublicInputs, Verifier } from "./verifiers";
import { checkRentExemption } from "./test-utils/testChecks";
import { MerkleTreeConfig } from "./merkleTree/merkleTreeConfig";
import {
  FIELD_SIZE,
  merkleTreeProgramId,
  Relayer,
  SolMerkleTree,
} from "./index";
import {
  MerkleTreeProgram,
  MerkleTreeProgramIdl,
} from "./idls/merkle_tree_program";

export type transactionParameters = {
  inputUtxos?: Array<Utxo>;
  outputUtxos?: Array<Utxo>;
  accounts: {
    sender?: PublicKey;
    recipient?: PublicKey;
    senderFee?: PublicKey;
    recipientFee?: PublicKey;
    verifierState?: PublicKey;
    tokenAuthority?: PublicKey;
    escrow?: PublicKey;
  };
  encryptedUtxos?: Uint8Array;
  verifier: Verifier;
  nullifierPdaPubkeys?: {
    isSigner: boolean;
    isWritable: boolean;
    pubkey: PublicKey;
  }[];
  leavesPdaPubkeys?: {
    isSigner: boolean;
    isWritable: boolean;
    pubkey: PublicKey;
  }[];
};

export class TransactionParameters implements transactionParameters {
  inputUtxos?: Array<Utxo>;
  outputUtxos?: Array<Utxo>;
  accounts: {
    sender?: PublicKey;
    recipient?: PublicKey;
    senderFee?: PublicKey;
    recipientFee?: PublicKey;
    verifierState?: PublicKey;
    tokenAuthority?: PublicKey;
    escrow?: PublicKey;
    systemProgramId: PublicKey;
    merkleTree: PublicKey;
    tokenProgram: PublicKey;
    registeredVerifierPda: PublicKey;
    authority: PublicKey;
    signingAddress?: PublicKey;
    preInsertedLeavesIndex: PublicKey;
    programMerkleTree: PublicKey;
  };
  encryptedUtxos?: Uint8Array;
  verifier: Verifier;
  verifierApp?: Verifier;
  nullifierPdaPubkeys?: {
    isSigner: boolean;
    isWritable: boolean;
    pubkey: PublicKey;
  }[];
  leavesPdaPubkeys?: {
    isSigner: boolean;
    isWritable: boolean;
    pubkey: PublicKey;
  }[];
  merkleTreeProgram?: Program<MerkleTreeProgramIdl>;

  constructor({
    merkleTreePubkey,
    verifier,
    sender,
    recipient,
    senderFee,
    recipientFee,
    inputUtxos,
    outputUtxos,
    verifierApp,
  }: {
    merkleTreePubkey: PublicKey;
    verifier: Verifier;
    verifierApp?: Verifier;
    sender?: PublicKey;
    recipient?: PublicKey;
    senderFee?: PublicKey;
    recipientFee?: PublicKey;
    inputUtxos?: Utxo[];
    outputUtxos?: Utxo[];
  }) {
    try {
      this.merkleTreeProgram = new Program(
        MerkleTreeProgram,
        merkleTreeProgramId,
      );
    } catch (error) {
      console.log(error);
      console.log("assuming test mode thus continuing");
      this.merkleTreeProgram = {
        programId: merkleTreeProgramId,
      };
    }

    this.accounts = {
      systemProgramId: SystemProgram.programId,
      tokenProgram: TOKEN_PROGRAM_ID,
      merkleTree: merkleTreePubkey,
      registeredVerifierPda: Transaction.getRegisteredVerifierPda(
        this.merkleTreeProgram.programId,
        verifier.verifierProgram.programId,
      ),
      authority: Transaction.getSignerAuthorityPda(
        this.merkleTreeProgram.programId,
        verifier.verifierProgram.programId,
      ),
      preInsertedLeavesIndex: PRE_INSERTED_LEAVES_INDEX,
      sender: sender,
      recipient: recipient,
      senderFee: senderFee,
      recipientFee: recipientFee,
      programMerkleTree: this.merkleTreeProgram.programId,
    };
    this.verifier = verifier;
    this.outputUtxos = outputUtxos;
    this.inputUtxos = inputUtxos;
    if (!this.outputUtxos && !inputUtxos) {
      throw new Error("No utxos provided.");
    }
    this.verifierApp = verifierApp;
  }
}

// TODO: make class
// TODO: add method getRelayer -> should return a rnd relayer from a list
export type LightInstance = {
  provider?: Provider;
  lookUpTable?: PublicKey;
  // TODO: build wrapper class SolMerkleTree around MerkleTree which includes
  //       merkle tree pubkey, buildMerkleTree, fetchLeaves
  solMerkleTree?: SolMerkleTree;
};

// add verifier class which is passed in with the constructor
// this class replaces the send transaction, also configures path the provingkey and witness, the inputs for the integrity hash
// input custom verifier with three functions by default prepare, proof, send
// include functions from sdk in shieldedTransaction

// TODO: add log option that enables logs
// TODO: write functional test for every method
export class Transaction {
  merkleTreeProgram?: Program<MerkleTreeProgramIdl>;
  payer?: SolanaKeypair;
  poseidon: any;
  shuffleEnabled: Boolean;
  action?: string;
  params?: TransactionParameters; // contains accounts
  appParams?: any;
  relayer: Relayer;
  instance: LightInstance;

  //txInputs;
  publicInputs?: PublicInputs;
  rootIndex: any;
  proofBytes: any;
  proofBytesApp: any;
  publicInputsApp?: PublicInputs;
  encryptedUtxos?: Uint8Array;

  proofInput: any;
  proofInputSystem: any;
  // Tmp rnd stuff for proof input
  assetPubkeysCircuit?: BN[];
  assetPubkeys?: PublicKey[];
  publicAmount?: BN;
  feeAmount?: BN;
  inputMerklePathIndices?: number[];
  inputMerklePathElements?: number[];
  publicInputsBytes?: number[][];
  connectingHash?: string;
  // Tests
  recipientBalancePriorTx?: BN;
  relayerRecipientAccountBalancePriorLastTx?: BN;

  /**
   * Initialize transaction
   *
   * @param instance encryptionKeypair used for encryption
   * @param relayer recipient of the unshielding
   * @param payer
   * @param shuffleEnabled
   */

  constructor({
    instance,
    relayer,
    payer,
    shuffleEnabled = true,
  }: {
    instance: LightInstance;
    relayer?: Relayer;
    payer?: SolanaKeypair;
    shuffleEnabled?: boolean;
  }) {
    if (relayer) {
      this.action = "WITHDRAWAL";
      this.relayer = relayer;
      this.payer = payer;
      console.log("withdrawal");
    } else if (!relayer && payer) {
      this.action = "DEPOSIT";
      this.payer = payer;
      this.relayer = new Relayer(payer.publicKey, instance.lookUpTable);
    } else {
      throw new Error("No payer and relayer provided.");
    }
    this.instance = instance;
    this.shuffleEnabled = shuffleEnabled;
  }

  // Returns serialized instructions
  async proveAndCreateInstructionsJson(
    params: TransactionParameters,
  ): Promise<string[]> {
    await this.compileAndProve(params);
    return await this.getInstructionsJson();
  }

  async proveAndCreateInstructions(
    params: TransactionParameters,
  ): Promise<TransactionInstruction[]> {
    await this.compileAndProve(params);
    return await this.params.verifier.getInstructions(this);
  }

  async compileAndProve(params: TransactionParameters) {
    await this.compile(params);
    await this.getProof();
  }

  async compile(params: TransactionParameters, appParams?: any) {
    // TODO: create and check for existence of merkleTreeAssetPubkey depending on utxo asset
    this.poseidon = await circomlibjs.buildPoseidonOpt();
    this.params = params;
    this.appParams = appParams;
    this.params.accounts.signingAddress = this.relayer.accounts.relayerPubkey;

    // prepare utxos
    const pubkeys = this.getAssetPubkeys(params.inputUtxos, params.outputUtxos);
    this.assetPubkeys = pubkeys.assetPubkeys;
    this.assetPubkeysCircuit = pubkeys.assetPubkeysCircuit;
    this.params.inputUtxos = this.addEmptyUtxos(
      params.inputUtxos,
      params.verifier.config.in,
    );
    this.params.outputUtxos = this.addEmptyUtxos(
      params.outputUtxos,
      params.verifier.config.out,
    );
    this.shuffleUtxos(this.params.inputUtxos);
    this.shuffleUtxos(this.params.outputUtxos);
    // prep and get proof inputs
    this.publicAmount = this.getExternalAmount(1);
    this.feeAmount = this.getExternalAmount(0);
    this.assignAccounts(params);
    this.getMerkleProofs();
    this.getProofInput();
    await this.getRootIndex();
  }

  getProofInput() {
    if (
      this.params &&
      this.instance.solMerkleTree.merkleTree &&
      this.params.inputUtxos &&
      this.params.outputUtxos &&
      this.assetPubkeysCircuit
    ) {
      this.proofInputSystem = {
        root: this.instance.solMerkleTree.merkleTree.root(),
        inputNullifier: this.params.inputUtxos.map((x) => x.getNullifier()),
        // TODO: move public and fee amounts into tx preparation
        publicAmount: this.getExternalAmount(1).toString(),
        feeAmount: this.getExternalAmount(0).toString(),
        extDataHash: this.getTxIntegrityHash().toString(),
        mintPubkey: this.assetPubkeysCircuit[1],
        inPrivateKey: this.params.inputUtxos?.map((x) => x.keypair.privkey),
        inPathIndices: this.inputMerklePathIndices,
        inPathElements: this.inputMerklePathElements,
      };
      this.proofInput = {
        outputCommitment: this.params.outputUtxos.map((x) => x.getCommitment()),
        inAmount: this.params.inputUtxos?.map((x) => x.amounts),
        inBlinding: this.params.inputUtxos?.map((x) => x.blinding),
        assetPubkeys: this.assetPubkeysCircuit,
        // data for 2 transaction outputUtxos
        outAmount: this.params.outputUtxos?.map((x) => x.amounts),
        outBlinding: this.params.outputUtxos?.map((x) => x.blinding),
        outPubkey: this.params.outputUtxos?.map((x) => x.keypair.pubkey),
        inIndices: this.getIndices(this.params.inputUtxos),
        outIndices: this.getIndices(this.params.outputUtxos),
        inInstructionType: this.params.inputUtxos?.map(
          (x) => x.instructionType,
        ),
        outInstructionType: this.params.outputUtxos?.map(
          (x) => x.instructionType,
        ),
        inPoolType: this.params.inputUtxos?.map((x) => x.poolType),
        outPoolType: this.params.outputUtxos?.map((x) => x.poolType),
        inVerifierPubkey: this.params.inputUtxos?.map(
          (x) => x.verifierAddressCircuit,
        ),
        outVerifierPubkey: this.params.outputUtxos?.map(
          (x) => x.verifierAddressCircuit,
        ),
      };
      if (this.appParams) {
        this.proofInput.connectingHash = this.getConnectingHash();
        this.proofInput.verifier = this.params.verifier?.pubkey;
      }
    } else {
      throw new Error(`getProofInput has undefined inputs`);
    }
  }

  async getAppProof() {
    if (this.appParams) {
      this.appParams.inputs.connectingHash = this.getConnectingHash();
      const path = require("path");
      // TODO: find a better more flexible solution
      const firstPath = path.resolve(__dirname, "../../../sdk/build-circuit/");
      let { proofBytes, publicInputs } = await this.getProofInternal(
        this.appParams.verifier,
        {
          ...this.appParams.inputs,
          ...this.proofInput,
          inPublicKey: this.params?.inputUtxos?.map(
            (utxo) => utxo.keypair.pubkey,
          ),
        },
        firstPath,
      );
      this.proofBytesApp = proofBytes;
      this.publicInputsApp = publicInputs;

      console.log("this.proofBytesApp ", this.proofBytesApp.toString());
      console.log("this.publicInputsApp ", this.publicInputsApp.toString());
    } else {
      throw new Error("No app params provided");
    }
  }

  async getProof() {
    const path = require("path");
    const firstPath = path.resolve(__dirname, "../build-circuits/");
    let { proofBytes, publicInputs } = await this.getProofInternal(
      this.params?.verifier,
      { ...this.proofInput, ...this.proofInputSystem },
      firstPath,
    );
    this.proofBytes = proofBytes;
    this.publicInputs = publicInputs;
    console.log();

    if (this.instance.provider) {
      await this.getPdaAddresses();
    }
  }

  async getProofInternal(verifier: Verifier, inputs: any, firstPath: string) {
    if (!this.instance.solMerkleTree?.merkleTree) {
      throw new Error("merkle tree not built");
    }
    if (!this.proofInput) {
      throw new Error("transaction not compiled");
    }
    if (!this.params) {
      throw new Error("params undefined probably not compiled");
    } else {
      const completePathWtns = firstPath + "/" + verifier.wtnsGenPath;
      const completePathZkey = firstPath + "/" + verifier.zkeyPath;
      const buffer = readFileSync(completePathWtns);

      let witnessCalculator = await verifier.calculateWtns(buffer);

      console.time("Proof generation");
      let wtns = await witnessCalculator.calculateWTNSBin(
        stringifyBigInts(inputs),
        0,
      );

      const { proof, publicSignals } = await snarkjs.groth16.prove(
        completePathZkey,
        wtns,
      );
      const proofJson = JSON.stringify(proof, null, 1);
      const publicInputsJson = JSON.stringify(publicSignals, null, 1);
      console.timeEnd("Proof generation");

      const vKey = await snarkjs.zKey.exportVerificationKey(completePathZkey);
      const res = await snarkjs.groth16.verify(vKey, publicSignals, proof);
      if (res === true) {
        console.log("Verification OK");
      } else {
        console.log("Invalid proof");
        throw new Error("Invalid Proof");
      }

      var publicInputsBytes = JSON.parse(publicInputsJson.toString());
      for (var i in publicInputsBytes) {
        publicInputsBytes[i] = Array.from(
          leInt2Buff(unstringifyBigInts(publicInputsBytes[i]), 32),
        ).reverse();
      }
      // console.log("publicInputsBytes ", publicInputsBytes);

      const proofBytes = await Transaction.parseProofToBytesArray(proofJson);

      const publicInputs =
        verifier.parsePublicInputsFromArray(publicInputsBytes);
      return { proofBytes, publicInputs };
      // await this.checkProof()
    }
  }

  getConnectingHash(): string {
    const inputHasher = this.poseidon.F.toString(
      this.poseidon(
        this.params?.inputUtxos?.map((utxo) => utxo.getCommitment()),
      ),
    );
    const outputHasher = this.poseidon.F.toString(
      this.poseidon(
        this.params?.outputUtxos?.map((utxo) => utxo.getCommitment()),
      ),
    );
    this.connectingHash = this.poseidon.F.toString(
      this.poseidon([inputHasher, outputHasher]),
    );
    return this.connectingHash;
  }

  assignAccounts(params: TransactionParameters) {
    if (this.assetPubkeys && this.params) {
      if (!this.params.accounts.sender && !this.params.accounts.senderFee) {
        if (this.action !== "WITHDRAWAL") {
          throw new Error("No relayer provided for withdrawal");
        }
        this.params.accounts.sender = MerkleTreeConfig.getSplPoolPdaToken(
          this.assetPubkeys[1],
          merkleTreeProgramId,
        );
        this.params.accounts.senderFee =
          MerkleTreeConfig.getSolPoolPda(merkleTreeProgramId).pda;

        if (!this.params.accounts.recipient) {
          this.params.accounts.recipient = SystemProgram.programId;
          if (this.publicAmount != new BN(0)) {
            throw new Error(
              "sth is wrong assignAccounts !params.accounts.recipient",
            );
          }
        }
        if (!this.params.accounts.recipientFee) {
          this.params.accounts.recipientFee = SystemProgram.programId;
          if (this.feeAmount != new BN(0)) {
            throw new Error(
              "sth is wrong assignAccounts !params.accounts.recipientFee",
            );
          }
        }
      } else {
        if (this.action !== "DEPOSIT") {
          throw new Error("Relayer should not be provided for deposit.");
        }

        this.params.accounts.recipient = MerkleTreeConfig.getSplPoolPdaToken(
          this.assetPubkeys[1],
          merkleTreeProgramId,
        );
        this.params.accounts.recipientFee =
          MerkleTreeConfig.getSolPoolPda(merkleTreeProgramId).pda;
        if (!this.params.accounts.sender) {
          this.params.accounts.sender = SystemProgram.programId;
          if (this.publicAmount != new BN(0)) {
            throw new Error(
              "sth is wrong assignAccounts !params.accounts.sender",
            );
          }
        }
        if (!this.params.accounts.senderFee) {
          this.params.accounts.senderFee = SystemProgram.programId;
          if (this.feeAmount != new BN(0)) {
            throw new Error(
              "sth is wrong assignAccounts !params.accounts.senderFee",
            );
          }
        }
      }
    } else {
      throw new Error("assignAccounts assetPubkeys undefined");
    }
  }

  getAssetPubkeys(
    inputUtxos?: Utxo[],
    outputUtxos?: Utxo[],
  ): { assetPubkeysCircuit: BN[]; assetPubkeys: PublicKey[] } {
    let assetPubkeysCircuit: BN[] = [
      hashAndTruncateToCircuit(SystemProgram.programId.toBytes()),
    ];

    let assetPubkeys: PublicKey[] = [SystemProgram.programId];

    if (inputUtxos) {
      inputUtxos.map((utxo) => {
        let found = false;
        for (var i in assetPubkeysCircuit) {
          if (
            assetPubkeysCircuit[i].toString() ===
            utxo.assetsCircuit[1].toString()
          ) {
            found = true;
          }
        }
        if (!found) {
          assetPubkeysCircuit.push(utxo.assetsCircuit[1]);
          assetPubkeys.push(utxo.assets[1]);
        }
      });
    }

    if (outputUtxos) {
      outputUtxos.map((utxo) => {
        let found = false;
        for (var i in assetPubkeysCircuit) {
          if (
            assetPubkeysCircuit[i].toString() ===
            utxo.assetsCircuit[1].toString()
          ) {
            found = true;
          }
        }
        if (!found) {
          assetPubkeysCircuit.push(utxo.assetsCircuit[1]);
          assetPubkeys.push(utxo.assets[1]);
        }
      });
    }

    if (assetPubkeys.length == 0) {
      throw new Error("No utxos provided.");
    }
    if (assetPubkeys.length > N_ASSET_PUBKEYS) {
      throw new Error("Utxos contain too many different assets.");
    }
    while (assetPubkeysCircuit.length < N_ASSET_PUBKEYS) {
      assetPubkeysCircuit.push(new BN(0));
    }

    return { assetPubkeysCircuit, assetPubkeys };
  }

  async getRootIndex() {
    if (this.instance.provider && this.instance.solMerkleTree.merkleTree) {
      this.merkleTreeProgram = new Program(
        MerkleTreeProgram,
        merkleTreeProgramId,
      );
      let root = Uint8Array.from(
        leInt2Buff(
          unstringifyBigInts(this.instance.solMerkleTree.merkleTree.root()),
          32,
        ),
      );
      let merkle_tree_account_data =
        await this.merkleTreeProgram.account.merkleTree.fetch(
          this.instance.solMerkleTree.pubkey,
        );

      merkle_tree_account_data.roots.map((x, index) => {
        if (x.toString() === root.toString()) {
          this.rootIndex = index;
        }
      });
    } else {
      console.log(
        "provider not defined did not fetch rootIndex set root index to 0",
      );
      this.rootIndex = 0;
    }
  }

  addEmptyUtxos(utxos: Utxo[] = [], len: number): Utxo[] {
    if (this.params && this.params.verifier.config) {
      while (utxos.length < len) {
        utxos.push(new Utxo({ poseidon: this.poseidon }));
      }
    } else {
      throw new Error(
        `input utxos ${utxos}, config ${this.params.verifier.config}`,
      );
    }
    return utxos;
  }

  // the fee plus the amount to pay has to be bigger than the amount in the input utxo
  // which doesn't make sense it should be the other way arround right
  // the external amount can only be made up of utxos of asset[0]
  // This might be too specific since the circuit allows assets to be in any index
  // TODO: write test
  getExternalAmount(assetIndex: number): BN {
    if (
      this.params &&
      this.params.inputUtxos &&
      this.params.outputUtxos &&
      this.assetPubkeysCircuit
    ) {
      return new anchor.BN(0)
        .add(
          this.params.outputUtxos
            .filter((utxo: Utxo) => {
              return (
                utxo.assetsCircuit[assetIndex].toString("hex") ==
                this.assetPubkeysCircuit[assetIndex].toString("hex")
              );
            })
            .reduce(
              (sum, utxo) =>
                // add all utxos of the same asset
                sum.add(utxo.amounts[assetIndex]),
              new anchor.BN(0),
            ),
        )
        .sub(
          this.params.inputUtxos
            .filter((utxo) => {
              return (
                utxo.assetsCircuit[assetIndex].toString("hex") ==
                this.assetPubkeysCircuit[assetIndex].toString("hex")
              );
            })
            .reduce(
              (sum, utxo) => sum.add(utxo.amounts[assetIndex]),
              new anchor.BN(0),
            ),
        )
        .add(FIELD_SIZE)
        .mod(FIELD_SIZE);
    } else {
      new Error(
        `this.params.inputUtxos ${this.params.inputUtxos} && this.params.outputUtxos ${this.params.outputUtxos} && this.assetPubkeysCircuit ${this.assetPubkeysCircuit}`,
      );
    }
  }

  // TODO: write test
  // TODO: make this work for edge case of two 2 different assets plus fee asset in the same transaction
  // TODO: fix edge case of an assetpubkey being 0
  getIndices(utxos: Utxo[]): string[][][] {
    let inIndices: string[][][] = [];

    utxos.map((utxo, index) => {
      let tmpInIndices = [];
      console.log("index ", index);
      console.log("inIndices ", inIndices);

      for (var a = 0; a < utxo.assets.length; a++) {
        console.log("a ", a);
        let tmpInIndices1: String[] = [];

        for (var i = 0; i < N_ASSET_PUBKEYS; i++) {
          try {
            console.log(i);

            console.log(
              `utxo ${utxo.assetsCircuit[
                a
              ].toString()} == ${this.assetPubkeysCircuit[i].toString()}`,
            );

            if (
              utxo.assetsCircuit[a].toString() ===
                this.assetPubkeysCircuit[i].toString() &&
              // utxo.amounts[a].toString() > "0" &&
              !tmpInIndices1.includes("1") &&
              this.assetPubkeysCircuit[i].toString() != "0"
            ) {
              // if (this.assetPubkeysCircuit[i].toString() == "0") {
              //   tmpInIndices1.push("0");

              // } else {
              //   tmpInIndices1.push("1");
              // }
              tmpInIndices1.push("1");
            } else {
              tmpInIndices1.push("0");
            }
          } catch (error) {
            tmpInIndices1.push("0");
          }
        }

        tmpInIndices.push(tmpInIndices1);
        console.log("tmpInIndices ", tmpInIndices);
      }

      inIndices.push(tmpInIndices);
    });
    console.log(inIndices);
    return inIndices;
  }

  getMerkleProofs() {
    this.inputMerklePathIndices = [];
    this.inputMerklePathElements = [];

    // getting merkle proofs
    for (const inputUtxo of this.params.inputUtxos) {
      if (
        inputUtxo.amounts[0] > new BN(0) ||
        inputUtxo.amounts[1] > new BN(0)
      ) {
        inputUtxo.index = this.instance.solMerkleTree.merkleTree.indexOf(
          inputUtxo.getCommitment(),
        );

        if (inputUtxo.index || inputUtxo.index == 0) {
          if (inputUtxo.index < 0) {
            throw new Error(
              `Input commitment ${inputUtxo.getCommitment()} was not found`,
            );
          }
          this.inputMerklePathIndices.push(inputUtxo.index);
          this.inputMerklePathElements.push(
            this.instance.solMerkleTree.merkleTree.path(inputUtxo.index)
              .pathElements,
          );
        }
      } else {
        this.inputMerklePathIndices.push(0);
        this.inputMerklePathElements.push(
          new Array(this.instance.solMerkleTree.merkleTree.levels).fill(0),
        );
      }
    }
  }

  getTxIntegrityHash(): BN {
    if (
      !this.params.accounts.recipient ||
      !this.params.accounts.recipientFee ||
      !this.relayer.relayerFee
    ) {
      throw new Error(
        `getTxIntegrityHash: recipient ${this.params.accounts.recipient} recipientFee ${this.params.accounts.recipientFee} relayerFee ${this.relayer.relayerFee}`,
      );
    } else {
      this.encryptedUtxos = this.encryptOutUtxos();
      if (this.encryptedUtxos && this.encryptedUtxos.length > 512) {
        this.encryptedUtxos = this.encryptedUtxos.slice(0, 512);
      }
      if (this.encryptedUtxos) {
        let extDataBytes = new Uint8Array([
          ...this.params.accounts.recipient?.toBytes(),
          ...this.params.accounts.recipientFee.toBytes(),
          ...this.payer.publicKey.toBytes(),
          ...this.relayer.relayerFee.toArray("le", 8),
          ...this.encryptedUtxos.slice(0, 512),
        ]);

        const hash = keccak_256
          .create({ dkLen: 32 })
          .update(Buffer.from(extDataBytes))
          .digest();
        return new anchor.BN(hash).mod(FIELD_SIZE);
      } else {
        throw new Error("Encrypting Utxos failed");
      }
    }
  }

  encryptOutUtxos(encryptedUtxos?: Uint8Array) {
    let encryptedOutputs = new Array<any>();
    if (encryptedUtxos) {
      encryptedOutputs = Array.from(encryptedUtxos);
    } else {
      this.params.outputUtxos.map((utxo, index) =>
        encryptedOutputs.push(utxo.encrypt()),
      );

      if (this.params.verifier.config.out == 2) {
        return new Uint8Array([
          ...encryptedOutputs[0],
          ...encryptedOutputs[1],
          ...new Array(256 - 190).fill(0),
          // this is ok because these bytes are not sent and just added for the integrity hash
          // to be consistent, if the bytes were sent to the chain use rnd bytes for padding
        ]);
      } else {
        let tmpArray = new Array<any>();
        for (var i = 0; i < this.params.verifier.config.out; i++) {
          tmpArray.push(...encryptedOutputs[i]);
          if (encryptedOutputs[i].length < 128) {
            // add random bytes for padding
            tmpArray.push(
              ...nacl.randomBytes(128 - encryptedOutputs[i].length),
            );
          }
        }

        if (tmpArray.length < 512) {
          tmpArray.push(
            new Array(
              this.params.verifier.config.out * 128 - tmpArray.length,
            ).fill(0),
          );
        }
        if (this.appParams.overwrite) {
          const utxoBytes =
            this.params?.outputUtxos[this.appParams.overwriteIndex].toBytes();
          tmpArray = this.overWriteEncryptedUtxos(utxoBytes, tmpArray, 0).slice(
            0,
            512,
          );
        }
        // return new Uint8Array(tmpArray.flat());
        return new Uint8Array([...tmpArray]);
      }
    }
  }

  // need this for the marketplace rn
  overWriteEncryptedUtxos(
    bytes: Uint8Array,
    toOverwriteBytes: Uint8Array,
    offSet: number = 0,
  ) {
    // this.encryptedUtxos.slice(offSet, bytes.length + offSet) = bytes;
    return Uint8Array.from([
      ...toOverwriteBytes.slice(0, offSet),
      ...bytes,
      ...toOverwriteBytes.slice(offSet + bytes.length, toOverwriteBytes.length),
    ]);
  }

  getPublicInputs() {
    this.publicInputs = this.params.verifier.parsePublicInputsFromArray(this);
  }

  // send transaction should be the same for both deposit and withdrawal
  // the function should just send the tx to the rpc or relayer respectively
  // in case there is more than one transaction to be sent to the verifier these can be sent separately

  async getTestValues() {
    try {
      this.recipientBalancePriorTx = (
        await getAccount(
          this.instance.provider.connection,
          this.params.accounts.recipient,
          TOKEN_PROGRAM_ID,
        )
      ).amount;
    } catch (e) {
      // covers the case of the recipient being a native sol address not a spl token address
      try {
        this.recipientBalancePriorTx =
          await this.instance.provider.connection.getBalance(
            this.params.accounts.recipient,
          );
      } catch (e) {}
    }
    try {
      this.recipientFeeBalancePriorTx =
        await this.instance.provider.connection.getBalance(
          this.params.accounts.recipientFee,
        );
    } catch (error) {
      console.log(
        "this.recipientFeeBalancePriorTx fetch failed ",
        this.params.accounts.recipientFee,
      );
    }

    this.senderFeeBalancePriorTx =
      await this.instance.provider.connection.getBalance(
        this.params.accounts.senderFee,
      );

    this.relayerRecipientAccountBalancePriorLastTx =
      await this.instance.provider.connection.getBalance(
        this.relayer.accounts.relayerRecipient,
      );
  }

  static getSignerAuthorityPda(
    merkleTreeProgramId: PublicKey,
    verifierProgramId: PublicKey,
  ) {
    return PublicKey.findProgramAddressSync(
      [merkleTreeProgramId.toBytes()],
      verifierProgramId,
    )[0];
  }
  static getRegisteredVerifierPda(
    merkleTreeProgramId: PublicKey,
    verifierProgramId: PublicKey,
  ) {
    return PublicKey.findProgramAddressSync(
      [verifierProgramId.toBytes()],
      merkleTreeProgramId,
    )[0];
  }

  async getInstructionsJson(): Promise<string[]> {
    if (!this.appParams) {
      const instructions = await this.params.verifier.getInstructions(this);
      let serialized = instructions.map((ix) => JSON.stringify(ix));
      return serialized;
    } else {
      const instructions = await this.appParams.verifier.getInstructions(this);
      let serialized = instructions.map((ix) => JSON.stringify(ix));
      return serialized;
    }
  }

  async sendTransaction(ix: any): Promise<TransactionSignature | undefined> {
    if (!this.payer) {
      // send tx to relayer
      let txJson = await this.getInstructionsJson();
      // request to relayer
      throw new Error("withdrawal with relayer is not implemented");
    } else {
      const recentBlockhash = (
        await this.instance.provider.connection.getRecentBlockhash("confirmed")
      ).blockhash;
      const txMsg = new TransactionMessage({
        payerKey: this.payer.publicKey,
        instructions: [
          ComputeBudgetProgram.setComputeUnitLimit({ units: 1_400_000 }),
          ix,
        ],
        recentBlockhash: recentBlockhash,
      });

      const lookupTableAccount =
        await this.instance.provider.connection.getAccountInfo(
          this.relayer.accounts.lookUpTable,
          "confirmed",
        );

      const unpackedLookupTableAccount = AddressLookupTableAccount.deserialize(
        lookupTableAccount.data,
      );

      const compiledTx = txMsg.compileToV0Message([
        {
          state: unpackedLookupTableAccount,
          key: this.relayer.accounts.lookUpTable,
          isActive: () => {
            return true;
          },
        },
      ]);

      compiledTx.addressTableLookups[0].accountKey =
        this.relayer.accounts.lookUpTable;

      const tx = new VersionedTransaction(compiledTx);
      let retries = 3;
      let res;
      while (retries > 0) {
        tx.sign([this.payer]);

        try {
          let serializedTx = tx.serialize();
          console.log("serializedTx: ");

          res = await this.instance.provider.connection.sendRawTransaction(
            serializedTx,
            confirmConfig,
          );
          retries = 0;
          console.log(res);
        } catch (e) {
          retries--;
          if (retries == 0 || e.logs != undefined) {
            console.log(e);
            return e;
          }
        }
      }
      return res;
    }
  }

  async getInstructions(): Promise<TransactionInstruction[]> {
    if (this.params) {
      return await this.params.verifier.getInstructions(this);
    } else {
      throw new Error("Params not provided.");
    }
  }

  async sendAndConfirmTransaction(): Promise<TransactionSignature> {
    if (!this.payer) {
      throw new Error("Cannot use sendAndConfirmTransaction without payer");
    }
    await this.getTestValues();
    var instructions;
    if (!this.appParams) {
      instructions = await this.params.verifier.getInstructions(this);
    } else {
      instructions = await this.appParams.verifier.getInstructions(this);
    }
    if (instructions) {
      let tx = "Something went wrong";
      for (var ix in instructions) {
        let txTmp = await this.sendTransaction(instructions[ix]);
        if (txTmp) {
          await this.instance.provider?.connection.confirmTransaction(
            txTmp,
            "confirmed",
          );
          tx = txTmp;
        } else {
          throw new Error("send transaction failed");
        }
      }
      return tx;
    } else {
      throw new Error("No parameters provided");
    }
  }

  async checkProof() {
    let publicSignals = [
      leBuff2int(Buffer.from(this.publicInputs.root.reverse())).toString(),
      leBuff2int(
        Buffer.from(this.publicInputs.publicAmount.reverse()),
      ).toString(),
      leBuff2int(
        Buffer.from(this.publicInputs.extDataHash.reverse()),
      ).toString(),
      leBuff2int(Buffer.from(this.publicInputs.feeAmount.reverse())).toString(),
      leBuff2int(
        Buffer.from(this.publicInputs.mintPubkey.reverse()),
      ).toString(),
      leBuff2int(
        Buffer.from(this.publicInputs.nullifiers[0].reverse()),
      ).toString(),
      leBuff2int(
        Buffer.from(this.publicInputs.nullifiers[1].reverse()),
      ).toString(),
      leBuff2int(Buffer.from(this.publicInputs.leaves[0].reverse())).toString(),
      leBuff2int(Buffer.from(this.publicInputs.leaves[1].reverse())).toString(),
    ];
    let pi_b_0 = this.proofBytes.slice(64, 128).reverse();
    let pi_b_1 = this.proofBytes.slice(128, 192).reverse();
    let proof = {
      pi_a: [
        leBuff2int(
          Buffer.from(this.proofBytes.slice(0, 32).reverse()),
        ).toString(),
        leBuff2int(
          Buffer.from(this.proofBytes.slice(32, 64).reverse()),
        ).toString(),
        "1",
      ],
      pi_b: [
        [
          leBuff2int(Buffer.from(pi_b_0.slice(0, 32))).toString(),
          leBuff2int(Buffer.from(pi_b_0.slice(32, 64))).toString(),
        ],
        [
          leBuff2int(Buffer.from(pi_b_1.slice(0, 32))).toString(),
          leBuff2int(Buffer.from(pi_b_1.slice(32, 64))).toString(),
        ],
        ["1", "0"],
      ],
      pi_c: [
        leBuff2int(
          Buffer.from(this.proofBytes.slice(192, 224).reverse()),
        ).toString(),
        leBuff2int(
          Buffer.from(this.proofBytes.slice(224, 256).reverse()),
        ).toString(),
        "1",
      ],
      protocol: "groth16",
      curve: "bn128",
    };

    const vKey = await snarkjs.zKey.exportVerificationKey(
      `${this.params.verifier.zkeyPath}.zkey`,
    );
    const res = await snarkjs.groth16.verify(vKey, publicSignals, proof);
    if (res === true) {
      console.log("Verification OK");
    } else {
      console.log("Invalid proof");
      throw new Error("Invalid Proof");
    }
  }

  async getPdaAddresses() {
    if (this.params && this.publicInputs && this.merkleTreeProgram) {
      let nullifiers = this.publicInputs.nullifiers;
      let merkleTreeProgram = this.merkleTreeProgram;
      let signer = this.relayer.accounts.relayerPubkey;

      this.params.nullifierPdaPubkeys = [];
      for (var i in nullifiers) {
        this.params.nullifierPdaPubkeys.push({
          isSigner: false,
          isWritable: true,
          pubkey: PublicKey.findProgramAddressSync(
            [Buffer.from(nullifiers[i]), anchor.utils.bytes.utf8.encode("nf")],
            merkleTreeProgram.programId,
          )[0],
        });
      }

      this.params.leavesPdaPubkeys = [];
      for (var i in this.publicInputs.leaves) {
        this.params.leavesPdaPubkeys.push({
          isSigner: false,
          isWritable: true,
          pubkey: PublicKey.findProgramAddressSync(
            [
              Buffer.from(Array.from(this.publicInputs.leaves[i][0]).reverse()),
              anchor.utils.bytes.utf8.encode("leaves"),
            ],
            merkleTreeProgram.programId,
          )[0],
        });
      }

      this.params.accounts.escrow = PublicKey.findProgramAddressSync(
        [anchor.utils.bytes.utf8.encode("escrow")],
        this.params.verifier.verifierProgram.programId,
      )[0];
      if (this.appParams) {
        this.params.accounts.verifierState = PublicKey.findProgramAddressSync(
          [signer.toBytes(), anchor.utils.bytes.utf8.encode("VERIFIER_STATE")],
          this.appParams.verifier.verifierProgram.programId,
        )[0];
      } else {
        this.params.accounts.verifierState = PublicKey.findProgramAddressSync(
          [signer.toBytes(), anchor.utils.bytes.utf8.encode("VERIFIER_STATE")],
          this.params.verifier.verifierProgram.programId,
        )[0];
      }

      this.params.accounts.tokenAuthority = PublicKey.findProgramAddressSync(
        [anchor.utils.bytes.utf8.encode("spl")],
        merkleTreeProgram.programId,
      )[0];
    } else {
      throw new Error(
        `${this.params} && ${this.publicInputs} && ${this.merkleTreeProgram}`,
      );
    }
  }

  async checkBalances() {
    // Checking that nullifiers were inserted
    this.is_token = true;

    for (var i in this.params.nullifierPdaPubkeys) {
      var nullifierAccount =
        await this.instance.provider.connection.getAccountInfo(
          this.params.nullifierPdaPubkeys[i].pubkey,
          {
            commitment: "confirmed",
          },
        );

      await checkRentExemption({
        account: nullifierAccount,
        connection: this.instance.provider.connection,
      });
    }
    let leavesAccount;
    var leavesAccountData;
    // Checking that leaves were inserted
    for (var i in this.params.leavesPdaPubkeys) {
      leavesAccountData =
        await this.merkleTreeProgram.account.twoLeavesBytesPda.fetch(
          this.params.leavesPdaPubkeys[i].pubkey,
        );

      assert(
        leavesAccountData.nodeLeft.toString() ==
          this.publicInputs.leaves[i][0].reverse().toString(),
        "left leaf not inserted correctly",
      );
      assert(
        leavesAccountData.nodeRight.toString() ==
          this.publicInputs.leaves[i][1].reverse().toString(),
        "right leaf not inserted correctly",
      );
      assert(
        leavesAccountData.merkleTreePubkey.toBase58() ==
          this.instance.solMerkleTree.pubkey.toBase58(),
        "merkleTreePubkey not inserted correctly",
      );

      for (var j = 0; j < this.encryptedUtxos.length / 256; j++) {
        // console.log(j);

        if (
          leavesAccountData.encryptedUtxos.toString() !==
          this.encryptedUtxos.toString()
        ) {
          // console.log(j);
          // throw `encrypted utxo ${i} was not stored correctly`;
        }
        // console.log(
        //   `${leavesAccountData.encryptedUtxos} !== ${this.encryptedUtxos}`
        // );

        // assert(leavesAccountData.encryptedUtxos === this.encryptedUtxos, "encryptedUtxos not inserted correctly");
        let decryptedUtxo1 = Utxo.decrypt({
          poseidon: this.poseidon,
          encBytes: this.encryptedUtxos,
          keypair: this.params.outputUtxos[0].keypair,
        });
        const utxoEqual = (utxo0: Utxo, utxo1: Utxo) => {
          assert.equal(
            utxo0.amounts[0].toString(),
            utxo1.amounts[0].toString(),
          );
          assert.equal(
            utxo0.amounts[1].toString(),
            utxo1.amounts[1].toString(),
          );
          assert.equal(utxo0.assets[0].toString(), utxo1.assets[0].toString());
          assert.equal(utxo0.assets[1].toString(), utxo1.assets[1].toString());
          assert.equal(
            utxo0.assetsCircuit[0].toString(),
            utxo1.assetsCircuit[0].toString(),
          );
          assert.equal(
            utxo0.assetsCircuit[1].toString(),
            utxo1.assetsCircuit[1].toString(),
          );
          assert.equal(
            utxo0.instructionType.toString(),
            utxo1.instructionType.toString(),
          );
          assert.equal(utxo0.poolType.toString(), utxo1.poolType.toString());
          assert.equal(
            utxo0.verifierAddress.toString(),
            utxo1.verifierAddress.toString(),
          );
          assert.equal(
            utxo0.verifierAddressCircuit.toString(),
            utxo1.verifierAddressCircuit.toString(),
          );
        };
        // console.log("decryptedUtxo ", decryptedUtxo1);
        // console.log("this.params.outputUtxos[0] ", this.params.outputUtxos[0]);

        utxoEqual(decryptedUtxo1, this.params.outputUtxos[0]);
      }
    }

    console.log(`mode ${this.action}, this.is_token ${this.is_token}`);

    try {
      var preInsertedLeavesIndexAccount =
        await this.instance.provider.connection.getAccountInfo(
          PRE_INSERTED_LEAVES_INDEX,
        );

      const preInsertedLeavesIndexAccountAfterUpdate =
        this.merkleTreeProgram.account.preInsertedLeavesIndex._coder.accounts.decode(
          "PreInsertedLeavesIndex",
          preInsertedLeavesIndexAccount.data,
        );
      console.log(
        "Number(preInsertedLeavesIndexAccountAfterUpdate.nextIndex) ",
        Number(preInsertedLeavesIndexAccountAfterUpdate.nextIndex),
      );
      console.log(
        `${Number(leavesAccountData.leftLeafIndex)} + ${
          this.params.leavesPdaPubkeys.length * 2
        }`,
      );

      assert(
        Number(preInsertedLeavesIndexAccountAfterUpdate.nextIndex) ==
          Number(leavesAccountData.leftLeafIndex) +
            this.params.leavesPdaPubkeys.length * 2,
      );
    } catch (e) {
      console.log("preInsertedLeavesIndex: ", e);
    }

    if (this.action == "DEPOSIT" && this.is_token == false) {
      var recipientAccount =
        await this.instance.provider.connection.getAccountInfo(
          this.params.accounts.recipient,
        );
      assert(
        recipientAccount.lamports ==
          I64(this.recipientBalancePriorTx)
            .add(this.publicAmount.toString())
            .toString(),
        "amount not transferred correctly",
      );
    } else if (this.action == "DEPOSIT" && this.is_token == true) {
      console.log("DEPOSIT and token");

      var recipientAccount = await getAccount(
        this.instance.provider.connection,
        this.params.accounts.recipient,
        TOKEN_PROGRAM_ID,
      );
      var recipientFeeAccountBalance =
        await this.instance.provider.connection.getBalance(
          this.params.accounts.recipientFee,
        );

      // console.log(`Balance now ${senderAccount.amount} balance beginning ${senderAccountBalancePriorLastTx}`)
      // assert(senderAccount.lamports == (I64(senderAccountBalancePriorLastTx) - I64.readLE(this.extAmount, 0)).toString(), "amount not transferred correctly");

      console.log(
        `Balance now ${recipientAccount.amount} balance beginning ${this.recipientBalancePriorTx}`,
      );
      console.log(
        `Balance now ${recipientAccount.amount} balance beginning ${
          Number(this.recipientBalancePriorTx) + Number(this.publicAmount)
        }`,
      );
      assert(
        recipientAccount.amount ==
          (
            Number(this.recipientBalancePriorTx) + Number(this.publicAmount)
          ).toString(),
        "amount not transferred correctly",
      );
      console.log(
        `Blanace now ${recipientFeeAccountBalance} ${
          Number(this.recipientFeeBalancePriorTx) + Number(this.feeAmount)
        }`,
      );
      console.log("fee amount: ", this.feeAmount);
      console.log(
        "fee amount from inputs. ",
        new anchor.BN(this.publicInputs.feeAmount.slice(24, 32)).toString(),
      );
      console.log(
        "pub amount from inputs. ",
        new anchor.BN(this.publicInputs.publicAmount.slice(24, 32)).toString(),
      );

      console.log(
        "recipientFeeBalancePriorTx: ",
        this.recipientFeeBalancePriorTx,
      );

      var senderFeeAccountBalance =
        await this.instance.provider.connection.getBalance(
          this.params.accounts.senderFee,
        );
      console.log("senderFeeAccountBalance: ", senderFeeAccountBalance);
      console.log(
        "this.senderFeeBalancePriorTx: ",
        this.senderFeeBalancePriorTx,
      );

      assert(
        recipientFeeAccountBalance ==
          Number(this.recipientFeeBalancePriorTx) + Number(this.feeAmount),
      );
      console.log(
        `${Number(this.senderFeeBalancePriorTx)} - ${Number(
          this.feeAmount,
        )} == ${senderFeeAccountBalance}`,
      );
      assert(
        Number(this.senderFeeBalancePriorTx) -
          Number(this.feeAmount) -
          5000 * this.params.verifier.instructions?.length ==
          Number(senderFeeAccountBalance),
      );
    } else if (this.action == "WITHDRAWAL" && this.is_token == false) {
      var senderAccount =
        await this.instance.provider.connection.getAccountInfo(
          this.params.accounts.sender,
        );
      var recipientAccount =
        await this.instance.provider.connection.getAccountInfo(
          this.params.accounts.recipient,
        );
      // console.log("senderAccount.lamports: ", senderAccount.lamports)
      // console.log("I64(senderAccountBalancePriorLastTx): ", I64(senderAccountBalancePriorLastTx).toString())
      // console.log("Sum: ", ((I64(senderAccountBalancePriorLastTx).add(I64.readLE(this.extAmount, 0))).sub(I64(relayerFee))).toString())

      assert.equal(
        senderAccount.lamports,
        I64(senderAccountBalancePriorLastTx)
          .add(I64.readLE(this.extAmount, 0))
          .sub(I64(relayerFee))
          .toString(),
        "amount not transferred correctly",
      );

      var recipientAccount =
        await this.instance.provider.connection.getAccountInfo(recipient);
      // console.log(`recipientAccount.lamports: ${recipientAccount.lamports} == sum ${((I64(Number(this.recipientBalancePriorTx)).sub(I64.readLE(this.extAmount, 0))).add(I64(relayerFee))).toString()}

      assert(
        recipientAccount.lamports ==
          I64(Number(this.recipientBalancePriorTx))
            .sub(I64.readLE(this.extAmount, 0))
            .toString(),
        "amount not transferred correctly",
      );
    } else if (this.action == "WITHDRAWAL" && this.is_token == true) {
      var senderAccount = await getAccount(
        this.instance.provider.connection,
        this.params.accounts.sender,
        TOKEN_PROGRAM_ID,
      );
      var recipientAccount = await getAccount(
        this.instance.provider.connection,
        this.params.accounts.recipient,
        TOKEN_PROGRAM_ID,
      );

      // assert(senderAccount.amount == ((I64(Number(senderAccountBalancePriorLastTx)).add(I64.readLE(this.extAmount, 0))).sub(I64(relayerFee))).toString(), "amount not transferred correctly");
      console.log(
        "this.recipientBalancePriorTx ",
        this.recipientBalancePriorTx,
      );
      console.log("this.publicAmount ", this.publicAmount);
      console.log(
        "this.publicAmount ",
        this.publicAmount?.sub(FIELD_SIZE).mod(FIELD_SIZE),
      );

      console.log(
        `${recipientAccount.amount}, ${new anchor.BN(
          this.recipientBalancePriorTx,
        )
          .sub(this.publicAmount?.sub(FIELD_SIZE).mod(FIELD_SIZE))
          .toString()}`,
      );
      assert.equal(
        recipientAccount.amount.toString(),
        new anchor.BN(this.recipientBalancePriorTx)
          .sub(this.publicAmount?.sub(FIELD_SIZE).mod(FIELD_SIZE))
          .toString(),
        "amount not transferred correctly",
      );

      var relayerAccount = await this.instance.provider.connection.getBalance(
        this.relayer.accounts.relayerRecipient,
      );

      var recipientFeeAccount =
        await this.instance.provider.connection.getBalance(
          this.params.accounts.recipientFee,
        );
      // console.log("recipientFeeAccount ", recipientFeeAccount);
      // console.log("this.feeAmount: ", this.feeAmount);
      // console.log(
      //   "recipientFeeBalancePriorTx ",
      //   this.recipientFeeBalancePriorTx
      // );

      console.log(
        `recipientFeeAccount ${new anchor.BN(recipientFeeAccount)
          .add(new anchor.BN(this.relayer.relayerFee.toString()))
          .add(new anchor.BN("5000"))
          .toString()} == ${new anchor.BN(this.recipientFeeBalancePriorTx)
          .sub(this.feeAmount?.sub(FIELD_SIZE).mod(FIELD_SIZE))
          .toString()}`,
      );

      // console.log("relayerAccount ", relayerAccount);
      // console.log("this.relayer.relayerFee: ", this.relayer.relayerFee);
      console.log(
        "relayerRecipientAccountBalancePriorLastTx ",
        this.relayerRecipientAccountBalancePriorLastTx,
      );
      console.log(
        `relayerFeeAccount ${new anchor.BN(relayerAccount)
          .sub(this.relayer.relayerFee)
          .toString()} == ${new anchor.BN(
          this.relayerRecipientAccountBalancePriorLastTx,
        )}`,
      );

      console.log(
        `relayerAccount ${new anchor.BN(
          relayerAccount,
        ).toString()} == ${new anchor.BN(
          this.relayerRecipientAccountBalancePriorLastTx,
        )
          .sub(new anchor.BN(this.relayer.relayerFee))
          .toString()}`,
      );

      console.log(
        `recipientFeeAccount ${new anchor.BN(recipientFeeAccount)
          .add(new anchor.BN(this.relayer.relayerFee.toString()))
          .toString()}  == ${new anchor.BN(this.recipientFeeBalancePriorTx)
          .sub(this.feeAmount?.sub(FIELD_SIZE).mod(FIELD_SIZE))
          .toString()}`,
      );

      assert.equal(
        new anchor.BN(recipientFeeAccount)
          .add(new anchor.BN(this.relayer.relayerFee.toString()))
          .toString(),
        new anchor.BN(this.recipientFeeBalancePriorTx)
          .sub(this.feeAmount?.sub(FIELD_SIZE).mod(FIELD_SIZE))
          .toString(),
      );
      // console.log(`this.relayer.relayerFee ${this.relayer.relayerFee} new anchor.BN(relayerAccount) ${new anchor.BN(relayerAccount)}`);

      assert.equal(
        new anchor.BN(relayerAccount)
          .sub(this.relayer.relayerFee)
          // .add(new anchor.BN("5000"))
          .toString(),
        this.relayerRecipientAccountBalancePriorLastTx?.toString(),
      );
    } else {
      throw Error("mode not supplied");
    }
  }

  // TODO: use higher entropy rnds
  shuffleUtxos(utxos: Utxo[]) {
    if (this.shuffleEnabled) {
      console.log("shuffling utxos");
    } else {
      console.log("commented shuffle");
      return;
    }
    let currentIndex: number = utxos.length;
    let randomIndex: number;

    // While there remain elements to shuffle...
    while (0 !== currentIndex) {
      // Pick a remaining element...
      randomIndex = Math.floor(Math.random() * currentIndex);
      currentIndex--;

      // And swap it with the current element.
      [utxos[currentIndex], utxos[randomIndex]] = [
        utxos[randomIndex],
        utxos[currentIndex],
      ];
    }

    return utxos;
  }

  // also converts lE to BE
  static async parseProofToBytesArray(data: any) {
    var mydata = JSON.parse(data.toString());

    for (var i in mydata) {
      if (i == "pi_a" || i == "pi_c") {
        for (var j in mydata[i]) {
          mydata[i][j] = Array.from(
            leInt2Buff(unstringifyBigInts(mydata[i][j]), 32),
          ).reverse();
        }
      } else if (i == "pi_b") {
        for (var j in mydata[i]) {
          for (var z in mydata[i][j]) {
            mydata[i][j][z] = Array.from(
              leInt2Buff(unstringifyBigInts(mydata[i][j][z]), 32),
            );
          }
        }
      }
    }
    return [
      mydata.pi_a[0],
      mydata.pi_a[1],
      mydata.pi_b[0].flat().reverse(),
      mydata.pi_b[1].flat().reverse(),
      mydata.pi_c[0],
      mydata.pi_c[1],
    ].flat();
  }
}
