import {
  PublicKey,
  SystemProgram,
  ComputeBudgetProgram,
  AddressLookupTableAccount,
  TransactionMessage,
  VersionedTransaction,
  TransactionSignature,
  TransactionInstruction,
} from "@solana/web3.js";
import { TOKEN_PROGRAM_ID, getAccount } from "@solana/spl-token";
import { BN, Program } from "@coral-xyz/anchor";
import {
  PRE_INSERTED_LEAVES_INDEX,
  confirmConfig,
  MERKLE_TREE_KEY,
} from "./constants";
import { N_ASSET_PUBKEYS, Utxo } from "./utxo";
import { PublicInputs, Verifier } from "./verifiers";
import { checkRentExemption } from "./test-utils/testChecks";
import { MerkleTreeConfig } from "./merkleTree/merkleTreeConfig";
import {
  FIELD_SIZE,
  hashAndTruncateToCircuit,
  Account,
  merkleTreeProgramId,
  Relayer,
} from "./index";
import { IDL_MERKLE_TREE_PROGRAM } from "./idls/index";
import { readFileSync } from "fs";
import { Provider as LightProvider } from "./wallet";
const anchor = require("@coral-xyz/anchor");
const snarkjs = require("snarkjs");
const nacl = require("tweetnacl");
var ffjavascript = require("ffjavascript");
const { unstringifyBigInts, stringifyBigInts, leInt2Buff, leBuff2int } =
  ffjavascript.utils;
const { keccak_256 } = require("@noble/hashes/sha3");
var assert = require("assert");

export const createEncryptionKeypair = () => nacl.box.keyPair();

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
  };
  relayer?: Relayer;
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
    systemProgramId: PublicKey;
    merkleTree: PublicKey;
    tokenProgram: PublicKey;
    registeredVerifierPda: PublicKey;
    authority: PublicKey;
    signingAddress?: PublicKey;
    preInsertedLeavesIndex: PublicKey;
    programMerkleTree: PublicKey;
  };
  relayer?: Relayer;
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
  merkleTreeProgram?: Program<typeof IDL_MERKLE_TREE_PROGRAM>;

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
    relayer,
    encryptedUtxos,
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
    relayer?: Relayer;
    encryptedUtxos?: Uint8Array;
  }) {
    try {
      this.merkleTreeProgram = new Program(
        IDL_MERKLE_TREE_PROGRAM,
        merkleTreeProgramId,
      );
    } catch (error) {
      console.log(error);
      console.log("assuming test mode thus continuing");
      this.merkleTreeProgram = {
        programId: merkleTreeProgramId,
      };
    }
    if (!this.merkleTreeProgram) throw new Error("merkleTreeProgram not set");
    if (!verifier) throw new Error("verifier undefined");
    if (!verifier.verifierProgram)
      throw new Error("verifier.verifierProgram undefined");

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
      sender: sender,
      recipient: recipient,
      senderFee: senderFee, // TODO: change to feeSender
      recipientFee: recipientFee, // TODO: change name to feeRecipient
      programMerkleTree: this.merkleTreeProgram.programId,
    };
    this.verifier = verifier;
    this.outputUtxos = outputUtxos;
    this.inputUtxos = inputUtxos;
    if (!this.outputUtxos && !inputUtxos) {
      throw new Error("No utxos provided.");
    }
    this.verifierApp = verifierApp;
    this.relayer = relayer;
    this.encryptedUtxos = encryptedUtxos;
  }
}

// add verifier class which is passed in with the constructor
// this class replaces the send transaction, also configures path the provingkey and witness, the inputs for the integrity hash
// input custom verifier with three functions by default prepare, proof, send
// include functions from sdk in shieldedTransaction

// TODO: add log option that enables logs
// TODO: write functional test for every method
export class Transaction {
  merkleTreeProgram?: Program<typeof IDL_MERKLE_TREE_PROGRAM>;
  shuffleEnabled: Boolean;
  action?: string;
  params?: TransactionParameters; // contains accounts
  appParams?: any;
  // TODO: relayer shd pls should be part of the provider by default + optional override on Transaction level
  provider: LightProvider;

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
  inputMerklePathElements?: string[][];
  publicInputsBytes?: number[][];
  connectingHash?: string;
  // Tests
  recipientBalancePriorTx?: BN;
  relayerRecipientAccountBalancePriorLastTx?: BN;
  txIntegrityHash?: BN;
  senderFeeBalancePriorTx?: BN;
  recipientFeeBalancePriorTx?: BN;
  is_token?: boolean;
  /**
   * Initialize transaction
   *
   * @param relayer recipient of the unshielding
   * @param shuffleEnabled
   */
  constructor({
    provider,
    shuffleEnabled = false,
  }: {
    provider: LightProvider;
    shuffleEnabled?: boolean;
  }) {
    if (!provider.poseidon) throw new Error("Poseidon not set");
    if (!provider.solMerkleTree) throw new Error("Merkle tree not set");
    if (!provider.browserWallet && !provider.nodeWallet)
      throw new Error("Wallet not set");
    this.provider = provider;

    this.shuffleEnabled = shuffleEnabled;
  }

  /** Returns serialized instructions */
  async proveAndCreateInstructionsJson(
    params: TransactionParameters,
  ): Promise<string[]> {
    await this.compileAndProve(params);
    return await this.getInstructionsJson();
  }

  async proveAndCreateInstructions(
    params: TransactionParameters,
    appParams?: any,
  ): Promise<TransactionInstruction[]> {
    await this.compileAndProve(params, appParams);
    if (appParams) {
      return await this.appParams.verifier.getInstructions(this);
    } else if (this.params) {
      return await this.params.verifier.getInstructions(this);
    } else {
      throw new Error("No parameters provided");
    }
  }

  async compileAndProve(params: TransactionParameters, appParams?: any) {
    await this.compile(params, appParams);
    await this.getProof();
    if (appParams) {
      await this.getAppProof();
    }
  }

  async compile(params: TransactionParameters, appParams?: any) {
    // TODO: create and check for existence of merkleTreeAssetPubkey depending on utxo asset
    this.params = params;
    this.appParams = appParams;

    if (params.relayer) {
      // TODO: rename to send
      this.action = "WITHDRAWAL";
      console.log("withdrawal");
    } else if (
      !params.relayer &&
      (this.provider.browserWallet || this.provider.nodeWallet) &&
      this.provider.lookUpTable
    ) {
      this.action = "DEPOSIT";
      this.params.relayer = new Relayer(
        this.provider.browserWallet
          ? this.provider.browserWallet.publicKey
          : this.provider.nodeWallet!.publicKey,
        this.provider.lookUpTable,
      );
    } else {
      throw new Error(
        "Couldn't assign relayer- no relayer nor wallet, or provider provided.",
      );
    }
    if (this.params.relayer) {
      this.params.accounts.signingAddress =
        this.params.relayer.accounts.relayerPubkey;
    } else {
      throw new Error(
        `Relayer not provided, or assigment failed at deposit this.params: ${this.params}`,
      );
    }

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
    this.assignAccounts();
    this.getMerkleProofs();
    this.getProofInput();
    await this.getRootIndex();
  }

  getMint() {
    if (this.getExternalAmount(1).toString() == "0") {
      return new BN(0);
    } else if (this.assetPubkeysCircuit) {
      return this.assetPubkeysCircuit[1];
    } else {
      throw new Error("Get mint failed");
    }
  }

  getProofInput() {
    if (
      this.params &&
      this.provider.solMerkleTree?.merkleTree &&
      this.params.inputUtxos &&
      this.params.outputUtxos &&
      this.assetPubkeysCircuit
    ) {
      this.proofInputSystem = {
        root: this.provider.solMerkleTree.merkleTree.root(),
        inputNullifier: this.params.inputUtxos.map((x) => x.getNullifier()),
        // TODO: move public and fee amounts into tx preparation
        publicAmount: this.getExternalAmount(1).toString(),
        feeAmount: this.getExternalAmount(0).toString(),
        mintPubkey: this.getMint(),
        inPrivateKey: this.params.inputUtxos?.map((x) => x.account.privkey),
        inPathIndices: this.inputMerklePathIndices,
        inPathElements: this.inputMerklePathElements,
      };
      this.proofInput = {
        extDataHash: this.getTxIntegrityHash().toString(),
        outputCommitment: this.params.outputUtxos.map((x) => x.getCommitment()),
        inAmount: this.params.inputUtxos?.map((x) => x.amounts),
        inBlinding: this.params.inputUtxos?.map((x) => x.blinding),
        assetPubkeys: this.assetPubkeysCircuit,
        // data for 2 transaction outputUtxos
        outAmount: this.params.outputUtxos?.map((x) => x.amounts),
        outBlinding: this.params.outputUtxos?.map((x) => x.blinding),
        outPubkey: this.params.outputUtxos?.map((x) => x.account.pubkey),
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
        this.proofInput.connectingHash = Transaction.getConnectingHash(
          this.params,
          this.provider.poseidon,
          this.proofInput.extDataHash, //this.getTxIntegrityHash().toString()
        );
        this.proofInput.verifier = this.params.verifier?.pubkey;
      }
    } else {
      throw new Error(`getProofInput has undefined inputs`);
    }
  }

  async getAppProof() {
    if (this.appParams && this.params) {
      this.appParams.inputs.connectingHash = Transaction.getConnectingHash(
        this.params,
        this.provider.poseidon,
        this.getTxIntegrityHash().toString(),
      );
      const path = require("path");
      // TODO: find a better more flexible solution
      const firstPath = path.resolve(__dirname, "../../../sdk/build-circuit/");
      let { proofBytes, publicInputs } = await this.getProofInternal(
        this.appParams.verifier,
        {
          ...this.appParams.inputs,
          ...this.proofInput,
          inPublicKey: this.params?.inputUtxos?.map(
            (utxo) => utxo.account.pubkey,
          ),
        },
        firstPath,
      );

      this.proofBytesApp = proofBytes;
      this.publicInputsApp = publicInputs;
    } else {
      throw new Error("No app params or params provided");
    }
  }

  async getProof() {
    const path = require("path");
    const firstPath = path.resolve(__dirname, "../build-circuits/");
    if (this.params && this.params?.verifier) {
      let { proofBytes, publicInputs } = await this.getProofInternal(
        this.params?.verifier,
        { ...this.proofInput, ...this.proofInputSystem },
        firstPath,
      );
      this.proofBytes = proofBytes;
      this.publicInputs = publicInputs;
    } else {
      throw new Error("Params not defined.");
    }

    // TODO: remove anchor provider if possible
    if (this.provider.provider) {
      await this.getPdaAddresses();
    }
  }

  async getProofInternal(verifier: Verifier, inputs: any, firstPath: string) {
    if (!this.provider.solMerkleTree?.merkleTree) {
      throw new Error("merkle tree not built");
    }
    if (!this.proofInput) {
      throw new Error("transaction not compiled");
    }
    if (!this.params) {
      throw new Error("params undefined probably not compiled");
    } else {
      // console.log("this.proofInput ", inputs);

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

      console.timeEnd("Proof generation");

      const vKey = await snarkjs.zKey.exportVerificationKey(completePathZkey);
      const res = await snarkjs.groth16.verify(vKey, publicSignals, proof);
      if (res === true) {
        console.log("Verification OK");
      } else {
        console.log("Invalid proof");
        throw new Error("Invalid Proof");
      }
      // const curve = await  ffjavascript.getCurveFromName(vKey.curve);
      // let neg_proof_a = curve.G1.neg(curve.G1.fromObject(proof.pi_a))
      // proof.pi_a = [
      //   ffjavascript.utils.stringifyBigInts(neg_proof_a.slice(0,32)).toString(),
      //     ffjavascript.utils.stringifyBigInts(neg_proof_a.slice(32,64)).toString(),
      //       '1'
      // ];
      const proofJson = JSON.stringify(proof, null, 1);
      const publicInputsJson = JSON.stringify(publicSignals, null, 1);

      var publicInputsBytesJson = JSON.parse(publicInputsJson.toString());
      var publicInputsBytes = new Array<Array<number>>();
      for (var i in publicInputsBytesJson) {
        let ref: Array<number> = Array.from([
          ...leInt2Buff(unstringifyBigInts(publicInputsBytesJson[i]), 32),
        ]).reverse();
        publicInputsBytes.push(ref);
        // TODO: replace ref, error is that le and be do not seem to be consistent
        // new BN(publicInputsBytesJson[i], "le").toArray("be",32)
        // assert.equal(ref.toString(), publicInputsBytes[publicInputsBytes.length -1].toString());
      }
      const publicInputs =
        verifier.parsePublicInputsFromArray(publicInputsBytes);

      const proofBytes = await Transaction.parseProofToBytesArray(proofJson);
      return { proofBytes, publicInputs };
    }
  }

  static getConnectingHash(
    params: TransactionParameters,
    poseidon: any,
    txIntegrityHash: any,
  ): string {
    const inputHasher = poseidon.F.toString(
      poseidon(params?.inputUtxos?.map((utxo) => utxo.getCommitment())),
    );
    const outputHasher = poseidon.F.toString(
      poseidon(params?.outputUtxos?.map((utxo) => utxo.getCommitment())),
    );
    const connectingHash = poseidon.F.toString(
      poseidon([inputHasher, outputHasher, txIntegrityHash.toString()]),
    );
    return connectingHash;
  }

  assignAccounts() {
    if (!this.params) throw new Error("Params undefined");
    if (!this.params.verifier.verifierProgram)
      throw new Error("Verifier.verifierProgram undefined");

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
          if (!this.publicAmount?.eq(new BN(0))) {
            throw new Error(
              "sth is wrong assignAccounts !params.accounts.recipient",
            );
          }
        }
        if (!this.params.accounts.recipientFee) {
          this.params.accounts.recipientFee = SystemProgram.programId;
          if (!this.feeAmount?.eq(new BN(0))) {
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
          if (!this.publicAmount?.eq(new BN(0))) {
            throw new Error(
              "sth is wrong assignAccounts !params.accounts.sender",
            );
          }
        }
        this.params.accounts.senderFee = PublicKey.findProgramAddressSync(
          [anchor.utils.bytes.utf8.encode("escrow")],
          this.params.verifier.verifierProgram.programId,
        )[0];
        // if (!this.params.accounts.senderFee) {

        //   if (!this.feeAmount?.eq(new BN(0))) {
        //     throw new Error(
        //       "sth is wrong assignAccounts !params.accounts.senderFee",
        //     );
        //   }
        // }
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
    if (!this.provider.solMerkleTree)
      throw new Error("provider.solMerkeTree not set");
    if (this.provider.provider && this.provider.solMerkleTree.merkleTree) {
      this.merkleTreeProgram = new Program(
        IDL_MERKLE_TREE_PROGRAM,
        merkleTreeProgramId,
      );
      let root = Uint8Array.from(
        leInt2Buff(
          unstringifyBigInts(this.provider.solMerkleTree.merkleTree.root()),
          32,
        ),
      );
      let merkle_tree_account_data =
        await this.merkleTreeProgram.account.merkleTree.fetch(
          this.provider.solMerkleTree.pubkey,
        );

      merkle_tree_account_data.roots.map((x: any, index: any) => {
        if (x.toString() === root.toString()) {
          this.rootIndex = index;
        }
      });

      if (this.rootIndex === undefined) {
        throw new Error(`Root index not found for root${root}`);
      }
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
        utxos.push(new Utxo({ poseidon: this.provider.poseidon }));
      }
    } else {
      throw new Error(
        `input utxos ${utxos}, config ${this.params?.verifier.config}`,
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
                this.assetPubkeysCircuit![assetIndex].toString("hex")
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
                this.assetPubkeysCircuit![assetIndex].toString("hex")
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
      throw new Error(
        `this.params.inputUtxos ${this.params?.inputUtxos} && this.params.outputUtxos ${this.params?.outputUtxos} && this.assetPubkeysCircuit ${this.assetPubkeysCircuit}`,
      );
    }
  }

  // TODO: make this work for edge case of two 2 different assets plus fee asset in the same transaction
  // TODO: fix edge case of an assetpubkey being 0
  // TODO: !== !! and check non-null
  getIndices(utxos: Utxo[]): string[][][] {
    let inIndices: string[][][] = [];
    if (this.assetPubkeysCircuit) {
      utxos.map((utxo, index) => {
        let tmpInIndices = [];
        for (var a = 0; a < utxo.assets.length; a++) {
          let tmpInIndices1: string[] = [];

          for (var i = 0; i < N_ASSET_PUBKEYS; i++) {
            try {
              if (
                utxo.assetsCircuit[a].toString() ===
                  this.assetPubkeysCircuit![i].toString() &&
                !tmpInIndices1.includes("1") &&
                this.assetPubkeysCircuit![i].toString() != "0"
              ) {
                tmpInIndices1.push("1");
              } else {
                tmpInIndices1.push("0");
              }
            } catch (error) {
              tmpInIndices1.push("0");
            }
          }

          tmpInIndices.push(tmpInIndices1);
        }

        inIndices.push(tmpInIndices);
      });
    } else {
      throw new Error("assetPubkeysCircuit undefined");
    }

    // console.log(inIndices);
    return inIndices;
  }

  getMerkleProofs() {
    this.inputMerklePathIndices = [];
    this.inputMerklePathElements = [];
    if (this.params && this.params.inputUtxos && this.provider.solMerkleTree) {
      // getting merkle proofs
      for (const inputUtxo of this.params.inputUtxos) {
        if (
          inputUtxo.amounts[0] > new BN(0) ||
          inputUtxo.amounts[1] > new BN(0)
        ) {
          inputUtxo.index = this.provider.solMerkleTree!.merkleTree.indexOf(
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
              this.provider.solMerkleTree.merkleTree.path(inputUtxo.index)
                .pathElements,
            );
          }
        } else {
          this.inputMerklePathIndices.push(0);
          this.inputMerklePathElements.push(
            new Array<string>(
              this.provider.solMerkleTree.merkleTree.levels,
            ).fill("0"),
          );
        }
      }
    }
  }

  getTxIntegrityHash(): BN {
    if (this.params && this.params.relayer) {
      if (
        !this.params.accounts.recipient ||
        !this.params.accounts.recipientFee ||
        !this.params.relayer.relayerFee
      ) {
        throw new Error(
          `getTxIntegrityHash: recipient ${this.params.accounts.recipient} recipientFee ${this.params.accounts.recipientFee} relayerFee ${this.params.relayer.relayerFee}`,
        );
      } else if (this.txIntegrityHash) {
        return this.txIntegrityHash;
      } else {
        if (!this.params.encryptedUtxos) {
          this.params.encryptedUtxos = this.encryptOutUtxos();
        }
        if (
          this.params.encryptedUtxos &&
          this.params.encryptedUtxos.length > 512
        ) {
          this.params.encryptedUtxos = this.params.encryptedUtxos.slice(0, 512);
        }
        if (this.params.encryptedUtxos && !this.txIntegrityHash) {
          let extDataBytes = new Uint8Array([
            ...this.params.accounts.recipient?.toBytes(),
            ...this.params.accounts.recipientFee.toBytes(),
            ...this.params.relayer.accounts.relayerPubkey.toBytes(),
            ...this.params.relayer.relayerFee.toArray("le", 8),
            ...this.params.encryptedUtxos,
          ]);

          const hash = keccak_256
            .create({ dkLen: 32 })
            .update(Buffer.from(extDataBytes))
            .digest();
          const txIntegrityHash: BN = new anchor.BN(hash).mod(FIELD_SIZE);
          this.txIntegrityHash = txIntegrityHash;
          return txIntegrityHash;
        } else {
          throw new Error("Encrypting Utxos failed");
        }
      }
    } else {
      throw new Error("params or relayer undefined");
    }
  }

  encryptOutUtxos(encryptedUtxos?: Uint8Array) {
    let encryptedOutputs = new Array<any>();
    if (encryptedUtxos) {
      encryptedOutputs = Array.from(encryptedUtxos);
    } else if (this.params && this.params.outputUtxos) {
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
            ...nacl.randomBytes(
              this.params.verifier.config.out * 128 - tmpArray.length,
            ),
            // new Array(
            //   this.params.verifier.config.out * 128 - tmpArray.length,
            // ).fill(0),
          );
        }
        return new Uint8Array([...tmpArray]);
      }
    }
  }

  // send transaction should be the same for both deposit and withdrawal
  // the function should just send the tx to the rpc or relayer respectively
  // in case there is more than one transaction to be sent to the verifier these can be sent separately

  async getTestValues() {
    if (!this.provider) {
      throw new Error("Provider undefined");
    }

    if (!this.provider.provider) {
      throw new Error("Provider.provider undefined");
    }

    if (!this.params) {
      throw new Error("params undefined");
    }

    if (!this.params.relayer) {
      throw new Error("params.relayer undefined");
    }

    if (!this.params.accounts.senderFee) {
      throw new Error("params.accounts.senderFee undefined");
    }

    if (!this.params.accounts.recipient) {
      throw new Error("params.accounts.recipient undefined");
    }

    if (!this.params.accounts.recipientFee) {
      throw new Error("params.accounts.recipient undefined");
    }

    try {
      this.recipientBalancePriorTx = new BN(
        (
          await getAccount(
            this.provider.provider.connection,
            this.params.accounts.recipient,
          )
        ).amount.toString(),
      );
    } catch (e) {
      // covers the case of the recipient being a native sol address not a spl token address
      try {
        this.recipientBalancePriorTx = new BN(
          await this.provider.provider.connection.getBalance(
            this.params.accounts.recipient,
          ),
        );
      } catch (e) {}
    }

    try {
      this.recipientFeeBalancePriorTx = new BN(
        await this.provider.provider.connection.getBalance(
          this.params.accounts.recipientFee,
        ),
      );
    } catch (error) {
      console.log(
        "this.recipientFeeBalancePriorTx fetch failed ",
        this.params.accounts.recipientFee,
      );
    }
    if (this.action === "DEPOSIT") {
      this.senderFeeBalancePriorTx = new BN(
        await this.provider.provider.connection.getBalance(
          this.params.relayer.accounts.relayerPubkey,
        ),
      );
    } else {
      this.senderFeeBalancePriorTx = new BN(
        await this.provider.provider.connection.getBalance(
          this.params.accounts.senderFee,
        ),
      );
    }

    this.relayerRecipientAccountBalancePriorLastTx = new BN(
      await this.provider.provider.connection.getBalance(
        this.params.relayer.accounts.relayerRecipient,
      ),
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
    if (!this.appParams && this.params) {
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
    if (false) {
      // TODO: replace this with (this.provider.browserWallet.pubkey != new relayer... this.relayer
      // then we know that an actual relayer was passed in and that it's supposed to be sent to one.
      // we cant do that tho as we'd want to add the default relayer to the provider itself.
      // so just pass in a flag here "shield, unshield, transfer" -> so devs don't have to know that it goes to a relayer.
      // send tx to relayer
      let txJson = await this.getInstructionsJson();
      // request to relayer
      throw new Error("withdrawal with relayer is not implemented");
    } else {
      if (!this.provider.provider) throw new Error("no provider set");
      if (!this.params) throw new Error("params undefined");
      if (!this.params.relayer) throw new Error("params.relayer undefined");

      const recentBlockhash = (
        await this.provider.provider.connection.getRecentBlockhash("confirmed")
      ).blockhash;
      const txMsg = new TransactionMessage({
        payerKey:
          this.params.relayer.accounts.relayerPubkey !==
            this.provider.browserWallet?.publicKey &&
          this.params.relayer.accounts.relayerPubkey !==
            this.provider.nodeWallet?.publicKey
            ? this.params.relayer.accounts.relayerPubkey
            : this.provider.browserWallet
            ? this.provider.browserWallet.publicKey
            : this.provider.nodeWallet!.publicKey,
        instructions: [
          ComputeBudgetProgram.setComputeUnitLimit({ units: 1_400_000 }),
          ix,
        ],
        recentBlockhash: recentBlockhash,
      });

      const lookupTableAccount =
        await this.provider.provider.connection.getAccountInfo(
          this.params.relayer.accounts.lookUpTable,
          "confirmed",
        );

      const unpackedLookupTableAccount = AddressLookupTableAccount.deserialize(
        lookupTableAccount!.data,
      );

      const compiledTx = txMsg.compileToV0Message([
        {
          state: unpackedLookupTableAccount,
          key: this.params.relayer.accounts.lookUpTable,
          isActive: () => {
            return true;
          },
        },
      ]);

      compiledTx.addressTableLookups[0].accountKey =
        this.params.relayer.accounts.lookUpTable;

      var tx = new VersionedTransaction(compiledTx);
      let retries = 3;
      let res;
      while (retries > 0) {
        if (this.provider.browserWallet) {
          // TODO: versiontx??
          console.error("versioned tx might throw here");
          tx = await this.provider.browserWallet.signTransaction(tx);
          // throw new Error(
          //   "versioned transaction in browser not implemented yet",
          // );
        } else {
          /** Just need to define relayer pubkey as signer a creation */
          tx.sign([this.provider.nodeWallet!]);
        }

        try {
          let serializedTx = tx.serialize();
          console.log("tx: ");

          res = await this.provider.provider.connection.sendRawTransaction(
            serializedTx,
            confirmConfig,
          );
          retries = 0;
          // console.log(res);
        } catch (e: any) {
          retries--;
          if (retries == 0 || e.logs !== undefined) {
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
    console.log(
      "browserwallet in sendAndConfirmTransaction?: ",
      this.provider.browserWallet,
    );
    if (!this.provider.nodeWallet && !this.provider.browserWallet) {
      throw new Error(
        "Cannot use sendAndConfirmTransaction without payer or browserWallet",
      );
    }
    await this.getTestValues();
    var instructions;
    if (!this.params) throw new Error("params undefined");

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
          console.log("tx ::", txTmp);
          await this.provider.provider?.connection.confirmTransaction(
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

  // TODO: deal with this: set own payer just for that? where is this used?
  async closeVerifierState(): Promise<TransactionSignature> {
    if (
      (this.provider.nodeWallet || this.provider.browserWallet) &&
      this.params &&
      !this.appParams
    ) {
      if (!this.params.verifier.verifierProgram)
        throw new Error("verifier.verifierProgram undefined");
      return await this.params?.verifier.verifierProgram.methods
        .closeVerifierState()
        .accounts({
          ...this.params.accounts,
        })
        .signers([this.provider.nodeWallet!]) // TODO: browserwallet? or only ever used by relayer?
        .rpc(confirmConfig);
    } else if (this.provider.nodeWallet && this.params && this.appParams) {
      return await this.appParams?.verifier.verifierProgram.methods
        .closeVerifierState()
        .accounts({
          ...this.params.accounts,
        })
        .signers([this.provider.nodeWallet])
        .rpc(confirmConfig);
    } else {
      throw new Error("No payer or params provided.");
    }
  }

  async getPdaAddresses() {
    if (!this.params) {
      throw new Error("this.params undefined");
    }

    if (!this.params.relayer) {
      throw new Error("this.params.relayer undefined");
    }

    if (!this.publicInputs) {
      throw new Error("this.publicInputs undefined");
    }

    if (!this.merkleTreeProgram) {
      throw new Error("this.merkleTreeProgram undefined");
    }
    if (!this.params.verifier.verifierProgram) {
      throw new Error("params.verifier.verifierProgram undefined");
    }

    let nullifiers = this.publicInputs.nullifiers;
    let merkleTreeProgram = this.merkleTreeProgram;
    let signer = this.params.relayer.accounts.relayerPubkey;

    this.params.nullifierPdaPubkeys = [];
    for (var i in nullifiers) {
      this.params.nullifierPdaPubkeys.push({
        isSigner: false,
        isWritable: true,
        pubkey: PublicKey.findProgramAddressSync(
          [
            Uint8Array.from([...nullifiers[i]]),
            anchor.utils.bytes.utf8.encode("nf"),
          ],
          merkleTreeProgram.programId,
        )[0],
      });
    }

    this.params.leavesPdaPubkeys = [];
    for (var j = 0; j < this.publicInputs.leaves.length; j++) {
      this.params.leavesPdaPubkeys.push({
        isSigner: false,
        isWritable: true,
        pubkey: PublicKey.findProgramAddressSync(
          [
            Buffer.from(Array.from(this.publicInputs.leaves[j][0]).reverse()),
            anchor.utils.bytes.utf8.encode("leaves"),
          ],
          merkleTreeProgram.programId,
        )[0],
      });
    }

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
  }

  // TODO: check why this is called encr keypair but account class
  async checkBalances(account?: Account) {
    if (!this.publicInputs) {
      throw new Error("public inputs undefined");
    }

    if (!this.params) {
      throw new Error("params undefined");
    }
    const checkUndefined = (variables: Array<any>) => {
      variables.map((v) => {
        if (!v) {
          throw new Error(`${Object.keys(v)[0]} undefined`);
        }
      });
    };

    if (!this.params) {
      throw new Error("params undefined");
    }

    if (!this.params.accounts.senderFee) {
      throw new Error("params.accounts.senderFee undefined");
    }

    if (!this.params.accounts.recipientFee) {
      throw new Error("params.accounts.recipientFee undefined");
    }

    if (!this.params.accounts.recipient) {
      throw new Error("params.accounts.recipient undefined");
    }

    if (!this.params.accounts.recipient) {
      throw new Error("params.accounts.recipient undefined");
    }

    if (!this.senderFeeBalancePriorTx) {
      throw new Error("senderFeeBalancePriorTx undefined");
    }

    if (!this.feeAmount) {
      throw new Error("feeAmount undefined");
    }

    if (!this.feeAmount) {
      throw new Error("feeAmount undefined");
    }

    if (!this.merkleTreeProgram) {
      throw new Error("merkleTreeProgram undefined");
    }
    this.provider.solMerkleTree;

    if (!this.provider) {
      throw new Error("provider undefined");
    }

    if (!this.provider.solMerkleTree) {
      throw new Error("provider.solMerkleTree undefined");
    }

    if (!this.params.encryptedUtxos) {
      throw new Error("params.encryptedUtxos undefined");
    }

    if (!this.params.outputUtxos) {
      throw new Error("params.outputUtxos undefined");
    }

    if (!this.provider.provider) {
      throw new Error("params.outputUtxos undefined");
    }

    if (!this.params.leavesPdaPubkeys) {
      throw new Error("params.leavesPdaPubkeys undefined");
    }

    if (!this.params.leavesPdaPubkeys) {
      throw new Error("params.leavesPdaPubkeys undefined");
    }

    if (!this.params.relayer) {
      throw new Error("params.relayer undefined");
    }

    if (!this.params.accounts.sender) {
      throw new Error("params.accounts.sender undefined");
    }
    if (!this.params.nullifierPdaPubkeys) {
      throw new Error("params.nullifierPdaPubkeys undefined");
    }

    // Checking that nullifiers were inserted
    if (new BN(this.proofInput.publicAmount).toString() === "0") {
      this.is_token = false;
    } else {
      this.is_token = true;
    }

    for (var i = 0; i < this.params.nullifierPdaPubkeys?.length; i++) {
      var nullifierAccount =
        await this.provider.provider!.connection.getAccountInfo(
          this.params.nullifierPdaPubkeys[i].pubkey,
          {
            commitment: "confirmed",
          },
        );

      await checkRentExemption({
        account: nullifierAccount,
        connection: this.provider.provider!.connection,
      });
    }
    let leavesAccount;
    var leavesAccountData;
    // Checking that leaves were inserted
    for (var i = 0; i < this.params.leavesPdaPubkeys.length; i++) {
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
          this.provider.solMerkleTree.pubkey.toBase58(),
        "merkleTreePubkey not inserted correctly",
      );

      for (var j = 0; j < this.params.encryptedUtxos.length / 256; j++) {
        // console.log(j);

        if (
          leavesAccountData.encryptedUtxos.toString() !==
          this.params.encryptedUtxos.toString()
        ) {
          // console.log(j);
          // throw `encrypted utxo ${i} was not stored correctly`;
        }
        // console.log(
        //   `${leavesAccountData.encryptedUtxos} !== ${this.params.encryptedUtxos}`
        // );

        // assert(leavesAccountData.encryptedUtxos === this.encryptedUtxos, "encryptedUtxos not inserted correctly");
        // TODO: add for both utxos of leafpda
        let decryptedUtxo1 = Utxo.decrypt({
          poseidon: this.provider.poseidon,
          encBytes: this.params!.encryptedUtxos!,
          account: account ? account : this.params!.outputUtxos![0].account,
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
        if (decryptedUtxo1 !== null) {
          utxoEqual(decryptedUtxo1, this.params.outputUtxos[0]);
        } else {
          console.log("Could not decrypt any utxo probably a withdrawal.");
        }
      }
    }

    console.log(`mode ${this.action}, this.is_token ${this.is_token}`);

    try {
      const merkleTreeAfterUpdate =
        await this.merkleTreeProgram.account.merkleTree.fetch(MERKLE_TREE_KEY);
      console.log(
        "Number(merkleTreeAfterUpdate.nextQueuedIndex) ",
        Number(merkleTreeAfterUpdate.nextQueuedIndex),
      );
      leavesAccountData =
        await this.merkleTreeProgram.account.twoLeavesBytesPda.fetch(
          this.params.leavesPdaPubkeys[0].pubkey,
        );
      console.log(
        `${Number(leavesAccountData.leftLeafIndex)} + ${
          this.params.leavesPdaPubkeys.length * 2
        }`,
      );

      assert.equal(
        Number(merkleTreeAfterUpdate.nextQueuedIndex),
        Number(leavesAccountData.leftLeafIndex) +
          this.params.leavesPdaPubkeys.length * 2,
      );
    } catch (e) {
      console.log("preInsertedLeavesIndex: ", e);
    }
    var nrInstructions;
    if (this.appParams) {
      nrInstructions = this.appParams.verifier.instructions?.length;
    } else if (this.params) {
      nrInstructions = this.params.verifier.instructions?.length;
    } else {
      throw new Error("No params provided.");
    }
    console.log("nrInstructions ", nrInstructions);

    if (this.action == "DEPOSIT" && this.is_token == false) {
      var recipientFeeAccountBalance =
        await this.provider.provider.connection.getBalance(
          this.params.accounts.recipientFee,
        );
      console.log(
        "recipientFeeBalancePriorTx: ",
        this.recipientFeeBalancePriorTx,
      );

      var senderFeeAccountBalance =
        await this.provider.provider.connection.getBalance(
          this.params.relayer.accounts.relayerPubkey,
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
        "this.senderFeeBalancePriorTx ",
        this.senderFeeBalancePriorTx,
      );

      console.log(
        `${new BN(this.senderFeeBalancePriorTx)
          .sub(this.feeAmount)
          .sub(new BN(5000 * nrInstructions))
          .toString()} == ${senderFeeAccountBalance}`,
      );
      assert(
        new BN(this.senderFeeBalancePriorTx)
          .sub(this.feeAmount)
          .sub(new BN(5000 * nrInstructions))
          .toString() == senderFeeAccountBalance.toString(),
      );
    } else if (this.action == "DEPOSIT" && this.is_token == true) {
      console.log("DEPOSIT and token");

      var recipientAccount = await getAccount(
        this.provider.provider.connection,
        this.params.accounts.recipient,
      );
      var recipientFeeAccountBalance =
        await this.provider.provider.connection.getBalance(
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
        recipientAccount.amount.toString() ===
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
        await this.provider.provider.connection.getBalance(
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
        `${new BN(this.senderFeeBalancePriorTx)
          .sub(this.feeAmount)
          .sub(new BN(5000 * nrInstructions))
          .toString()} == ${senderFeeAccountBalance}`,
      );
      assert(
        new BN(this.senderFeeBalancePriorTx)
          .sub(this.feeAmount)
          .sub(new BN(5000 * nrInstructions))
          .toString() == senderFeeAccountBalance.toString(),
      );
    } else if (this.action == "WITHDRAWAL" && this.is_token == false) {
      var relayerAccount = await this.provider.provider.connection.getBalance(
        this.params.relayer.accounts.relayerRecipient,
      );

      var recipientFeeAccount =
        await this.provider.provider.connection.getBalance(
          this.params.accounts.recipientFee,
        );

      // console.log("relayerAccount ", relayerAccount);
      // console.log("this.params.relayer.relayerFee: ", this.params.relayer.relayerFee);
      console.log(
        "relayerRecipientAccountBalancePriorLastTx ",
        this.relayerRecipientAccountBalancePriorLastTx,
      );
      console.log(
        `relayerFeeAccount ${new anchor.BN(relayerAccount)
          .sub(this.params.relayer.relayerFee)
          .toString()} == ${new anchor.BN(
          this.relayerRecipientAccountBalancePriorLastTx,
        )}`,
      );
      console.log("SWEN: rfa", recipientFeeAccount);
      console.log(
        `recipientFeeAccount ${new anchor.BN(recipientFeeAccount)
          .add(new anchor.BN(this.params.relayer.relayerFee.toString()))
          .toString()}  == ${new anchor.BN(this.recipientFeeBalancePriorTx)
          .sub(this.feeAmount?.sub(FIELD_SIZE).mod(FIELD_SIZE))
          .toString()}`,
      );

      assert.equal(
        new anchor.BN(recipientFeeAccount)
          .add(new anchor.BN(this.params.relayer.relayerFee.toString()))
          .toString(),
        new anchor.BN(this.recipientFeeBalancePriorTx)
          .sub(this.feeAmount?.sub(FIELD_SIZE).mod(FIELD_SIZE))
          .toString(),
      );
      // console.log(`this.params.relayer.relayerFee ${this.params.relayer.relayerFee} new anchor.BN(relayerAccount) ${new anchor.BN(relayerAccount)}`);
      assert.equal(
        new anchor.BN(relayerAccount)
          .sub(this.params.relayer.relayerFee)
          .toString(),
        this.relayerRecipientAccountBalancePriorLastTx?.toString(),
      );
    } else if (this.action == "WITHDRAWAL" && this.is_token == true) {
      var senderAccount = await getAccount(
        this.provider.provider.connection,
        this.params.accounts.sender,
      );

      var recipientAccount = await getAccount(
        this.provider.provider.connection,
        this.params.accounts.recipient,
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

      var relayerAccount = await this.provider.provider.connection.getBalance(
        this.params.relayer.accounts.relayerRecipient,
      );

      var recipientFeeAccount =
        await this.provider.provider.connection.getBalance(
          this.params.accounts.recipientFee,
        );

      // console.log("relayerAccount ", relayerAccount);
      // console.log("this.params.relayer.relayerFee: ", this.params.relayer.relayerFee);
      console.log(
        "relayerRecipientAccountBalancePriorLastTx ",
        this.relayerRecipientAccountBalancePriorLastTx,
      );
      console.log(
        `relayerFeeAccount ${new anchor.BN(relayerAccount)
          .sub(this.params.relayer.relayerFee)
          .toString()} == ${new anchor.BN(
          this.relayerRecipientAccountBalancePriorLastTx,
        )}`,
      );

      console.log(
        `recipientFeeAccount ${new anchor.BN(recipientFeeAccount)
          .add(new anchor.BN(this.params.relayer.relayerFee.toString()))
          .toString()}  == ${new anchor.BN(this.recipientFeeBalancePriorTx)
          .sub(this.feeAmount?.sub(FIELD_SIZE).mod(FIELD_SIZE))
          .toString()}`,
      );

      assert.equal(
        new anchor.BN(recipientFeeAccount)
          .add(new anchor.BN(this.params.relayer.relayerFee.toString()))
          .toString(),
        new anchor.BN(this.recipientFeeBalancePriorTx)
          .sub(this.feeAmount?.sub(FIELD_SIZE).mod(FIELD_SIZE))
          .toString(),
      );

      assert.equal(
        new anchor.BN(relayerAccount)
          .sub(this.params.relayer.relayerFee)
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
      console.log("shuffle disabled");
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
    return {
      proofA: [mydata.pi_a[0], mydata.pi_a[1]].flat(),
      proofB: [
        mydata.pi_b[0].flat().reverse(),
        mydata.pi_b[1].flat().reverse(),
      ].flat(),
      proofC: [mydata.pi_c[0], mydata.pi_c[1]].flat(),
    };
  }
}
