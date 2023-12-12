import { BN, Program, utils } from "@coral-xyz/anchor";
import {
  Action,
  confirmConfig,
  Provider,
  REGISTERED_POOL_PDA_SOL,
  Relayer,
  SolMerkleTree,
  Transaction,
  TransactionParameters,
  Account,
  Utxo,
  ADMIN_AUTH_KEYPAIR,
  IDL_LIGHT_PSP4IN4OUT_APP_STORAGE,
  MerkleTreeConfig,
} from "@lightprotocol/zk.js";
import { MultisigParams } from "./multisigParams";
import { Scalar } from "ffjavascript";
// import boxen from 'boxen';
import {
  Keypair as SolanaKeypair,
  SystemProgram,
  PublicKey,
} from "@solana/web3.js";
// import { MERKLE_TREE_KEY } from "light-sdk";
// import { MockVerifier } from "./verifier";
const path = require("path");
import { IDL, Multisig as MultisigProgram } from "./types/multisig";

import { verifierProgramId, MAX_SIGNERS } from "./constants";
import { QueuedTransaction, Approval } from "./transaction";
import { Hasher } from "@lightprotocol/account.rs";

/**
 * Data:
 * - input Utxos 4 * 128 = 512
 * - output Utxos 4 * 128 = 512
 * - encryptedUtxos up to 512
 * - recipientSpl 32
 * - recipientSol 32
 * - relayerPubkey 32
 * - relayerFee 8
 */
export class MultiSigClient {
  signer: Account;
  multiSigParams: MultisigParams;
  hasher: Hasher;
  poseidon: any;
  eddsa: any;
  queuedTransactions: QueuedTransaction[];
  provider: Provider;
  verifier: Program<MultisigProgram>;
  constructor({
    multiSigParams,
    signer,
    hasher,
    poseidon,
    queuedTransactions,
    eddsa,
    provider,
  }: {
    multiSigParams: MultisigParams;
    signer: Account;
    hasher: Hasher;
    poseidon: any;
    queuedTransactions?: QueuedTransaction[];
    eddsa: any;
    provider: Provider;
  }) {
    this.multiSigParams = multiSigParams;
    this.signer = signer;
    this.hasher = hasher;
    this.eddsa = eddsa;
    if (queuedTransactions) {
      this.queuedTransactions = queuedTransactions;
    } else {
      this.queuedTransactions = [];
    }
    this.provider = provider;
    this.verifier = new Program(IDL, verifierProgramId);
  }

  // load call to load multisig

  // getMultiSig

  // TODO: need to enforce correct order in signatures relative to instructionDataHash
  // getApprovals

  // getQueuedTransactions()

  async approve(index: number) {
    // let tx = new Transaction({
    //   provider: this.provider,
    //   shuffleEnabled: false,
    //   params: this.queuedTransactions[index].transactionParams,
    //   appParams: { mock: "1231" },
    // });
    // await tx.compile();

    this.provider.solMerkleTree!.getMerkleProofs(
      this.hasher,
      this.queuedTransactions[index].transactionParams.inputUtxos,
    );
    const integrityHash = await this.queuedTransactions[
      index
    ].transactionParams.getTxIntegrityHash(this.hasher);

    const connectingHash = this.queuedTransactions[
      index
    ].transactionParams.getTransactionHash(this.hasher);

    const publicKey = await this.signer.getEddsaPublicKey();
    const signature = await this.signer.signEddsa(
      this.poseidon.F.e(Scalar.e(connectingHash)),
    );
    const approval = new Approval({
      signerIndex: index,
      publicKey,
      signature,
    });
    this.queuedTransactions[index].approvals.push(approval);
    console.log("\n\n------------------------------------------");
    console.log("\t Approved Multisig Transaction ");
    console.log("------------------------------------------");
    console.log(
      "The Approval is encrypted to the shared encryption key and stored in a (compressed) account on Solana.\n",
    );
    console.log(
      "Signer: ",
      utils.bytes.hex.encode(
        Buffer.from(
          Array.from([
            ...this.queuedTransactions[index].approvals[
              this.queuedTransactions[index].approvals.length - 1
            ].publicKey[0],
            ...this.queuedTransactions[index].approvals[
              this.queuedTransactions[index].approvals.length - 1
            ].publicKey[1],
          ]).flat(),
        ),
      ),
    );
    console.log("Shielded transaction hash: ", connectingHash.toString());
    console.log(
      "Signature: ",
      utils.bytes.hex.encode(
        Buffer.from(
          this.queuedTransactions[index].approvals[
            this.queuedTransactions[index].approvals.length - 1
          ].signature,
        ),
      ),
    );
    console.log("------------------------------------------\n");

    return this.queuedTransactions[index];
  }

  // approve and broadcast

  static async createMultiSigParameters(
    threshold: number,
    signer: Account,
    signers: Account[],
    hasher: Hasher,
    poseidon: any | undefined,
    eddsa: any | undefined,
    provider: Provider,
  ) {
    const multisig = await MultisigParams.createNewMultiSig({
      hasher,
      signers,
      threshold,
    });
    return new MultiSigClient({
      multiSigParams: multisig,
      signer,
      hasher,
      poseidon,
      provider,
      eddsa,
    });
  }

  createUtxo({
    splAsset,
    splAmount,
    solAmount,
  }: {
    splAsset?: PublicKey;
    splAmount?: BN;
    solAmount?: BN;
  }) {
    const appData = {
      threshold: this.multiSigParams.threshold,
      nrSigners: this.multiSigParams.nrSigners,
      publicKeyX: this.multiSigParams.publicKeyX.map(
        (s) => new BN(this.poseidon.F.toString(s)), //.toArrayLike(Buffer, "be", 32)
      ),
      publicKeyY: this.multiSigParams.publicKeyY.map(
        (s) => new BN(this.poseidon.F.toString(s)), //.toArrayLike(Buffer, "be", 32)
      ),
    };

    if (splAmount && splAsset) {
      let realSolAmount = new BN(0);
      if (solAmount) {
        realSolAmount = solAmount;
      }
      return new Utxo({
        hasher: this.hasher,
        assets: [SystemProgram.programId, splAsset],
        publicKey: this.multiSigParams.account.pubkey,
        amounts: [realSolAmount, splAmount],
        appData,
        appDataIdl: IDL,
        verifierAddress: verifierProgramId,
        assetLookupTable: this.provider.lookUpTables.assetLookupTable,
      });
    } else if (solAmount) {
      return new Utxo({
        hasher: this.hasher,
        assets: [SystemProgram.programId],
        publicKey: this.multiSigParams.account.pubkey,
        amounts: [solAmount],
        appData,
        appDataIdl: IDL,
        verifierAddress: verifierProgramId,
        assetLookupTable: this.provider.lookUpTables.assetLookupTable,
      });
    } else {
      throw new Error("Provided invalid params to create multisig createUtxo");
    }
  }

  // creates a transaction and queues it internally
  async createMultiSigTransaction({
    inputUtxos,
    outputUtxos,
    relayer,
    recipientSpl = SolanaKeypair.generate().publicKey,
    recipientSol = SolanaKeypair.generate().publicKey,
    sender = SolanaKeypair.generate().publicKey,
    action,
  }: {
    sender?: PublicKey;
    recipientSpl?: PublicKey;
    recipientSol?: PublicKey;
    inputUtxos: Utxo[];
    outputUtxos: Utxo[];
    relayer: Relayer;
    action: Action;
  }) {
    let encryptedUtxos = [];
    for (let utxo of outputUtxos) {
      let encryptedUtxo = await utxo.encrypt(this.poseidon);
      encryptedUtxos.push(encryptedUtxo);
    }

    const txParams = new TransactionParameters({
      inputUtxos,
      outputUtxos,
      transactionMerkleTreePubkey: MerkleTreeConfig.getTransactionMerkleTreePda(
        new BN(0),
      ),
      eventMerkleTreePubkey: MerkleTreeConfig.getEventMerkleTreePda(new BN(0)),
      recipientSol,
      recipientSpl,
      action,
      hasher: this.hasher,
      relayer,
      verifierIdl: IDL_LIGHT_PSP4IN4OUT_APP_STORAGE,
      encryptedUtxos: new Uint8Array([
        ...encryptedUtxos.flat(),
        ...new Array(512 - encryptedUtxos.flat().length).fill(1),
      ]),
      account: this.multiSigParams.account,
    });

    this.queuedTransactions.push(new QueuedTransaction(txParams));
    return txParams;
  }

  // TODO: implement serialize deserialize for transaction params
  // TODO: implement Create and Broadcast

  async createAppParams(index: number) {
    const keypairDummy = new Account({
      hasher: this.hasher,
      seed: new Uint8Array(32).fill(3).toString(),
      eddsa: this.eddsa,
    });

    let pubkeyDummy = await keypairDummy.getEddsaPublicKey();
    pubkeyDummy[0] = new Uint8Array(32).fill(0);
    pubkeyDummy[1] = new Uint8Array(32).fill(0);

    const signatureDummy = new Uint8Array(64).fill(0); //await keypairDummy.signEddsa("123");

    for (
      let i = this.queuedTransactions[index].approvals.length;
      i < MAX_SIGNERS;
      i++
    ) {
      this.queuedTransactions[index].approvals.push(
        new Approval({
          signerIndex: index, //TODO: fix this
          publicKey: pubkeyDummy,
          signature: signatureDummy,
        }),
      );
    }

    const circuitPath = path.join("build-circuit");

    const appParams = {
      inputs: {
        isAppInUtxo: undefined,
        threshold: this.multiSigParams.threshold.toString(),
        nrSigners: this.multiSigParams.nrSigners.toString(),
        signerPubkeysX: this.queuedTransactions[index].approvals.map(
          (approval) =>
            this.poseidon.F.toObject(approval.publicKey[0]).toString(),
        ),
        signerPubkeysY: this.queuedTransactions[index].approvals.map(
          (approval) =>
            this.poseidon.F.toObject(approval.publicKey[1]).toString(),
        ),
        enabled: [1, 1, ...new Array(MAX_SIGNERS - 2).fill(0)],
        signatures: this.queuedTransactions[index].approvals.map(
          (approval) => this.eddsa.unpackSignature(approval.signature).S,
        ),

        r8x: this.queuedTransactions[index].approvals.map((approval) =>
          this.poseidon.F.toObject(
            this.eddsa.unpackSignature(approval.signature).R8[0],
          ),
        ),
        r8y: this.queuedTransactions[index].approvals.map((approval) =>
          this.poseidon.F.toObject(
            this.eddsa.unpackSignature(approval.signature).R8[1],
          ),
        ),
      },
      verifierIdl: IDL,
      path: circuitPath,
    };
    return appParams;
  }

  static getAppInUtxoIndices(appUtxos: Utxo[]): number[] {
    let isAppInUtxo = [];
    for (const i in appUtxos) {
      let array = new Array(4).fill(new BN(0));
      if (appUtxos[i].appData) {
        array[i] = new BN(1);
        isAppInUtxo.push(array);
      }
    }
    return isAppInUtxo;
  }

  async execute(index: number) {
    const appParams = await this.createAppParams(index);
    let params = this.queuedTransactions[0].transactionParams;
    appParams.inputs.isAppInUtxo = MultiSigClient.getAppInUtxoIndices(
      params.inputUtxos,
    );

    let { rootIndex, remainingAccounts } = await this.provider.getRootIndex();
    let tx = new Transaction({
      shuffleEnabled: false,
      rootIndex,
      ...remainingAccounts,
      solMerkleTree: this.provider.solMerkleTree!,
      params: params,
      appParams,
    });

    const instructions = await tx.compileAndProve(
      this.poseidon,
      params.account,
    );
    await this.provider.sendAndConfirmTransaction(instructions);

    // await tx.checkBalances();
  }
}

export const printUtxo = (
  utxo: Utxo,
  poseidon: any,
  index: number,
  input: string,
) => {
  let string = `-------------- ${input} Utxo ${index} --------------\n`;
  string += `Amount sol: ${utxo.amounts[0]} \n`;
  string += `Amount spl: ${
    utxo.amounts[1]
  }, mint spl: ${utxo.assets[1].toBase58()}\n`;
  string += `Shielded pubkey: ${utxo.publicKey.toString("hex")}\n`;
  string += `Commitment: ${utxo.getCommitment(poseidon)}\n`;
  string += `Verifier pubkey: ${utxo.verifierAddress.toBase58()}\n`;
  string += `Instruction hash: ${utxo.appDataHash.toString()}\n`;
  string += "------------------------------------------";
  return string;
};
