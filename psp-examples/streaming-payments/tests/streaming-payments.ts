import * as anchor from "@coral-xyz/anchor";
import { BN } from "@coral-xyz/anchor";
import { assert } from "chai";
import {
  Account,
  Action,
  airdropSol,
  confirmConfig,
  ConfirmOptions,
  IDL_LIGHT_PSP4IN4OUT_APP_STORAGE,
  MerkleTreeConfig,
  ProgramUtxoBalance,
  Provider as LightProvider,
  TestRelayer,
  User,
  Utxo,
  ProgramParameters,
  createProofInputs,
  setUndefinedPspCircuitInputsToZero,
  PspTransactionInput,
  getSystemProof,
  SolanaTransactionInputs,
  sendAndConfirmShieldedTransaction,
  createTransaction,
  lightPsp4in4outAppStorageId,
  getVerifierProgramId,
  shieldProgramUtxo,
} from "@lightprotocol/zk.js";
import { Hasher, WasmHasher } from "@lightprotocol/account.rs";
import {
  Keypair as SolanaKeypair,
  Keypair,
  PublicKey,
  SystemProgram,
} from "@solana/web3.js";
import { IDL } from "../target/types/streaming_payments";
import { MerkleTree } from "@lightprotocol/circuit-lib.js";

const path = require("path");

const verifierProgramId = new PublicKey(
  "Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS",
);
let HASHER: Hasher;

const RPC_URL = "http://127.0.0.1:8899";
const USERS_COUNT = 3;

const users = new Array(USERS_COUNT).fill(null).map(() => {
  return {
    wallet: Keypair.generate(),
    relayerRecipientSol: SolanaKeypair.generate().publicKey,
  };
});

describe("Streaming Payments tests", () => {
  process.env.ANCHOR_PROVIDER_URL = RPC_URL;
  process.env.ANCHOR_WALLET = process.env.HOME + "/.config/solana/id.json";

  // Configure the client to use the local cluster.
  const provider = anchor.AnchorProvider.local(RPC_URL, confirmConfig);
  anchor.setProvider(provider);

  before(async () => {
    HASHER = await WasmHasher.getInstance();
  });

  it("Create and Spend Program Utxo for one user", async () => {
    await createAndSpendProgramUtxo(
      users[0].wallet,
      users[0].relayerRecipientSol,
    );
  });

  it.skip(`Create and Spend Program Utxo for ${users.length} users`, async () => {
    const logLabel = "Create and Spend Program Utxo for ${users.length} users";
    console.time(logLabel);
    let calls = [];
    for (const user of users) {
      calls.push(
        createAndSpendProgramUtxo(user.wallet, user.relayerRecipientSol),
      );
    }
    await Promise.all(calls);
    console.timeEnd(logLabel);
  });

  it.skip("Payment streaming", async () => {
    await paymentStreaming(users[0].wallet, users[0].relayerRecipientSol);
  });

  it.skip(`Payment streaming for ${users.length} users`, async () => {
    const logLabel = "Payment streaming for ${users.length} users";
    console.time(logLabel);
    let calls = [];
    for (const user of users) {
      calls.push(paymentStreaming(user.wallet, user.relayerRecipientSol));
    }
    await Promise.all(calls);
    console.timeEnd(logLabel);
  });
  async function createAndSpendProgramUtxo(
    wallet: anchor.web3.Keypair,
    relayerRecipientSol: anchor.web3.PublicKey,
  ): Promise<void> {
    await airdropSol({
      connection: provider.connection,
      lamports: 1e9,
      recipientPublicKey: wallet.publicKey,
    });

    await airdropSol({
      connection: provider.connection,
      lamports: 1e9,
      recipientPublicKey: relayerRecipientSol,
    });
    let relayer = new TestRelayer({
      relayerPubkey: wallet.publicKey,
      relayerRecipientSol: relayerRecipientSol,
      relayerFee: new BN(100_000),
      payer: wallet,
      connection: provider.connection,
      hasher: HASHER,
    });

    // The light provider is a connection and wallet abstraction.
    // The wallet is used to derive the seed for your shielded keypair with a signature.
    const lightProvider = await LightProvider.init({
      wallet,
      url: RPC_URL,
      relayer,
      confirmConfig,
    });
    const lightUser: User = await User.init({ provider: lightProvider });

    const outputUtxoSol = new Utxo({
      hasher: HASHER,
      assets: [SystemProgram.programId],
      publicKey: lightUser.account.keypair.publicKey,
      amounts: [new BN(1_000_000)],
      appData: { endSlot: new BN(1), rate: new BN(1) },
      appDataIdl: IDL,
      verifierAddress: verifierProgramId,
      assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
      includeAppData: true,
    });
    const testInputsShield = {
      utxo: outputUtxoSol,
      action: Action.SHIELD,
    };

    const storeProgramUtxoResult = await shieldProgramUtxo({
      account: lightUser.account,
      appUtxo: testInputsShield.utxo,
      provider: lightProvider,
    });
    console.log("storeProgramUtxoResult: ", storeProgramUtxoResult);
    const programUtxoBalance: Map<string, ProgramUtxoBalance> =
      await lightUser.syncStorage(IDL);
    const shieldedUtxoCommitmentHash =
      testInputsShield.utxo.getCommitment(HASHER);
    const inputUtxo = programUtxoBalance
      .get(verifierProgramId.toBase58())
      .tokenBalances.get(testInputsShield.utxo.assets[0].toBase58())
      .utxos.get(shieldedUtxoCommitmentHash);
    Utxo.equal(HASHER, inputUtxo, testInputsShield.utxo, false);

    const circuitPath = path.join(
      "build-circuit/streaming-payments/streamingPayments",
    );
    // TODO: add in and out utxos to appParams
    // TODO: create compile appParams method which creates isAppIn and out utxo arrays, prefixes utxo data variables with in and out prefixes
    const pspTransactionInput: PspTransactionInput = {
      proofInputs: {
        currentSlotPrivate: new BN(1),
        currentSlot: new BN(1),
      },
      path: circuitPath,
      verifierIdl: IDL,
      circuitName: "streamingPayments",
      // ts-ignore
      checkedInUtxos: [{ utxoName: "streamInUtxo", utxo: inputUtxo }],
    };

    const shieldedTransaction = await createTransaction({
      inputUtxos: [inputUtxo],
      transactionMerkleTreePubkey: MerkleTreeConfig.getTransactionMerkleTreePda(
        new BN(0),
      ),
      relayerPublicKey: relayer.accounts.relayerPubkey,
      hasher: HASHER,
      relayerFee: relayer.relayerFee,
      pspId: verifierProgramId,
      systemPspId: lightPsp4in4outAppStorageId,
      account: lightUser.account,
    });
    // createProofInputsAndProve
    const { root, index: rootIndex } = (await relayer.getMerkleRoot(
      MerkleTreeConfig.getTransactionMerkleTreePda(),
    ))!;
    const proofInputs = createProofInputs({
      hasher: HASHER,
      transaction: shieldedTransaction,
      pspTransaction: pspTransactionInput,
      account: lightUser.account,
      root,
    });

    const systemProof = await getSystemProof({
      account: lightUser.account,
      systemProofInputs: proofInputs,
      verifierIdl: IDL_LIGHT_PSP4IN4OUT_APP_STORAGE,
      inputUtxos: shieldedTransaction.private.inputUtxos,
    });

    const completePspProofInputs = setUndefinedPspCircuitInputsToZero(
      proofInputs,
      IDL,
      pspTransactionInput.circuitName,
    );

    const pspProof = await lightUser.account.getProofInternal({
      firstPath: pspTransactionInput.path,
      verifierIdl: pspTransactionInput.verifierIdl,
      proofInput: completePspProofInputs,
      inputUtxos: [inputUtxo],
    });

    const solanaTransactionInputs: SolanaTransactionInputs = {
      action: Action.TRANSFER,
      systemProof,
      pspProof,
      publicTransactionVariables: shieldedTransaction.public,
      pspTransactionInput,
      relayerRecipientSol: relayer.accounts.relayerRecipientSol,
      eventMerkleTree: MerkleTreeConfig.getEventMerkleTreePda(),
      systemPspIdl: IDL_LIGHT_PSP4IN4OUT_APP_STORAGE,
      rootIndex,
    };

    const res = await sendAndConfirmShieldedTransaction({
      solanaTransactionInputs,
      provider: lightProvider,
    });
    console.log("tx Hash : ", res.txHash);
  }

  async function paymentStreaming(
    wallet: anchor.web3.Keypair,
    relayerRecipientSol: anchor.web3.PublicKey,
  ) {
    const circuitPath = path.join("build-circuit");
    await airdropSol({
      connection: provider.connection,
      lamports: 1e10,
      recipientPublicKey: wallet.publicKey,
    });

    await airdropSol({
      connection: provider.connection,
      lamports: 1e10,
      recipientPublicKey: relayerRecipientSol,
    });

    let relayer = new TestRelayer({
      relayerPubkey: wallet.publicKey,
      relayerRecipientSol: relayerRecipientSol,
      relayerFee: new BN(100_000),
      payer: wallet,
      connection: provider.connection,
      hasher: HASHER,
    });

    // The light provider is a connection and wallet abstraction.
    // The wallet is used to derive the seed for your shielded keypair with a signature.
    const lightProvider = await LightProvider.init({
      wallet,
      url: RPC_URL,
      relayer,
      confirmConfig,
    });
    const lightUser: User = await User.init({ provider: lightProvider });

    let client: PaymentStreamClient = new PaymentStreamClient(
      IDL,
      HASHER,
      circuitPath,
      lightProvider,
    );
    const currentSlot = await provider.connection.getSlot("confirmed");
    const duration = 1;
    const streamInitUtxo = client.setupSolStream(
      new BN(1e9),
      new BN(duration),
      new BN(currentSlot),
      lightUser.account,
    );

    const testInputsSol1 = {
      utxo: streamInitUtxo,
      action: Action.SHIELD,
      hasher: HASHER,
    };

    console.log("storing streamInitUtxo");

    await shieldProgramUtxo({
      account: lightUser.account,
      appUtxo: testInputsSol1.utxo,
      provider: lightProvider,
    });
    await lightUser.syncStorage(IDL);
    const commitment = testInputsSol1.utxo.getCommitment(testInputsSol1.hasher);

    const utxo = (await lightUser.getUtxo(commitment))!;
    assert.equal(utxo.status, "ready");
    Utxo.equal(HASHER, utxo.utxo, testInputsSol1.utxo, true);
    const currentSlot1 = await provider.connection.getSlot("confirmed");

    await lightUser.getBalance();
    let merkleTree = (lightUser.provider.relayer as any).solMerkleTree
      .merkleTree;

    const { programParameters, inUtxo, outUtxo, action } = client.collectStream(
      new BN(currentSlot1),
      Action.TRANSFER,
      merkleTree,
    );
    // @ts-ignore: this code is not maintained and the api does not exist anymore
    await lightUser.executeAppUtxo({
      appUtxos: [inUtxo],
      programParameters,
      action,
      confirmOptions: ConfirmOptions.spendable,
    });
    const balance = await lightUser.getBalance();
    console.log(
      "totalSolBalance: ",
      balance.totalSolBalance.toNumber() * 1e-9,
      "SOL",
    );
    assert.equal(
      outUtxo.amounts[0].toString(),
      balance.totalSolBalance.toString(),
    );
    console.log("inUtxo commitment: ", inUtxo.getCommitment(HASHER));

    const spentCommitment = testInputsSol1.utxo.getCommitment(
      testInputsSol1.hasher,
    );
    const utxoSpent = (await lightUser.getUtxo(
      spentCommitment,
      true,
      MerkleTreeConfig.getTransactionMerkleTreePda(),
      IDL,
    ))!;
    assert.equal(utxoSpent.status, "spent");
  }
});

class PaymentStreamClient {
  idl: anchor.Idl;
  endSlot?: BN;
  streamInitUtxo?: Utxo;
  latestStreamUtxo?: Utxo;
  hasher: Hasher;
  circuitPath: string;
  lightProvider: LightProvider;

  constructor(
    idl: anchor.Idl,
    hasher: Hasher,
    circuitPath: string,
    lightProvider: LightProvider,
    streamInitUtxo?: Utxo,
    latestStreamUtxo?: Utxo,
  ) {
    this.idl = idl;
    this.streamInitUtxo = streamInitUtxo;
    this.endSlot = streamInitUtxo?.appData.endSlot;
    this.latestStreamUtxo = latestStreamUtxo;
    this.hasher = hasher;
    this.circuitPath = circuitPath;
    this.lightProvider = lightProvider;
  }
  /**
   * Creates a streamProgramUtxo
   * @param amount
   * @param timeInSlots
   * @param currentSlot
   * @param account
   */
  setupSolStream(
    amount: BN,
    timeInSlots: BN,
    currentSlot: BN,
    account: Account,
  ) {
    if (this.streamInitUtxo)
      throw new Error("This stream client is already initialized");

    const endSlot = currentSlot.add(timeInSlots);
    this.endSlot = endSlot;
    const rate = amount.div(timeInSlots);
    const appData = {
      endSlot,
      rate,
    };
    const streamInitUtxo = new Utxo({
      hasher: this.hasher,
      assets: [SystemProgram.programId],
      publicKey: account.keypair.publicKey,
      amounts: [amount],
      appData: appData,
      appDataIdl: this.idl,
      verifierAddress: getVerifierProgramId(this.idl),
      assetLookupTable: this.lightProvider.lookUpTables.assetLookupTable,
    });

    this.streamInitUtxo = streamInitUtxo;
    this.latestStreamUtxo = streamInitUtxo;
    return streamInitUtxo;
  }

  collectStream(currentSlot: BN, action: Action, merkleTree: MerkleTree) {
    if (!this.streamInitUtxo)
      throw new Error(
        "Streaming client is not initialized with streamInitUtxo",
      );
    if (currentSlot.gte(this.streamInitUtxo?.appData.endSlot)) {
      const currentSlotPrivate = this.streamInitUtxo.appData.endSlot;
      const diff = currentSlot.sub(currentSlotPrivate);
      const programParameters: ProgramParameters = {
        inputs: {
          currentSlotPrivate,
          currentSlot,
          diff,
          remainingAmount: new BN(0),
          isOutUtxo: new Array(4).fill(0),
          ...this.streamInitUtxo.appData,
        },
        verifierIdl: IDL,
        path: this.circuitPath,
        circuitName: "streamingPayments",
      };

      const index = merkleTree.indexOf(
        this.latestStreamUtxo?.getCommitment(this.hasher),
      );
      this.latestStreamUtxo.index = index;
      const inUtxo = this.latestStreamUtxo;
      if (action === Action.TRANSFER) {
        const outUtxo = new Utxo({
          assets: inUtxo.assets,
          amounts: [inUtxo.amounts[0].sub(new BN(100_000)), inUtxo.amounts[1]],
          publicKey: inUtxo.publicKey,
          hasher: this.hasher,
          assetLookupTable: this.lightProvider.lookUpTables.assetLookupTable,
        });
        return { programParameters, inUtxo, outUtxo, action };
      }
      return { programParameters, inUtxo, action };
    } else {
      const remainingAmount = this.streamInitUtxo.appData?.endSlot
        .sub(currentSlot)
        .mul(this.streamInitUtxo.appData?.rate);
      const programParameters: ProgramParameters = {
        inputs: {
          currentSlotPrivate: currentSlot,
          currentSlot,
          diff: new BN(0),
          remainingAmount: new BN(0),
          isOutUtxo: [1, 0, 0, 0],
          endSlot: this.endSlot,
          ...this.streamInitUtxo.appData,
        },
        verifierIdl: IDL,
        path: this.circuitPath,
        circuitName: "streamingPayments",
      };
      const inUtxo = this.latestStreamUtxo;
      const outUtxo = new Utxo({
        hasher: this.hasher,
        assets: [SystemProgram.programId],
        publicKey: inUtxo.publicKey,
        amounts: [remainingAmount],
        appData: this.streamInitUtxo.appData,
        appDataIdl: this.idl,
        verifierAddress: getVerifierProgramId(this.idl),
        assetLookupTable: this.lightProvider.lookUpTables.assetLookupTable,
      });
      return { programParameters, outUtxo, inUtxo };
    }
  }
}
