import * as anchor from "@coral-xyz/anchor";
import { BN } from "@coral-xyz/anchor";
import {
  Action,
  airdropSol,
  confirmConfig,
  IDL_LIGHT_PSP4IN4OUT_APP_STORAGE,
  MERKLE_TREE_SET,
  MerkleTreeConfig,
  ProgramUtxoBalance,
  Provider as LightProvider,
  TestRpc,
  User,
  createProofInputs,
  setUndefinedPspCircuitInputsToZero,
  PspTransactionInput,
  getSystemProof,
  SolanaTransactionInputs,
  sendAndConfirmCompressedTransaction,
  createTransaction,
  lightPsp4in4outAppStorageId,
  compressProgramUtxo,
  createProgramOutUtxo,
} from "@lightprotocol/zk.js";
import { LightWasm, WasmFactory } from "@lightprotocol/account.rs";
import { compareOutUtxos } from "../../../zk.js/tests/test-utils/compareUtxos";
import {
  Keypair as SolanaKeypair,
  Keypair,
  PublicKey,
  SystemProgram,
} from "@solana/web3.js";
import { IDL } from "../target/types/streaming_payments";
import { createDataHashWithDefaultHashingSchema } from "@lightprotocol/zk.js";

const path = require("path");

const verifierProgramId = new PublicKey(
  "Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS"
);
let WASM: LightWasm;

const RPC_URL = "http://127.0.0.1:8899";
const USERS_COUNT = 3;

const users = new Array(USERS_COUNT).fill(null).map(() => {
  return {
    wallet: Keypair.generate(),
    rpcRecipientSol: SolanaKeypair.generate().publicKey,
  };
});

describe("Streaming Payments tests", () => {
  process.env.ANCHOR_PROVIDER_URL = RPC_URL;
  process.env.ANCHOR_WALLET = process.env.HOME + "/.config/solana/id.json";

  // Configure the client to use the local cluster.
  const provider = anchor.AnchorProvider.local(RPC_URL, confirmConfig);
  anchor.setProvider(provider);

  before(async () => {
    WASM = await WasmFactory.getInstance();
  });

  it("Create and Spend Program Utxo for one user", async () => {
    await createAndSpendProgramUtxo(users[0].wallet, users[0].rpcRecipientSol);
  });

  async function createAndSpendProgramUtxo(
    wallet: anchor.web3.Keypair,
    rpcRecipientSol: anchor.web3.PublicKey
  ): Promise<void> {
    await airdropSol({
      connection: provider.connection,
      lamports: 1e9,
      recipientPublicKey: wallet.publicKey,
    });

    await airdropSol({
      connection: provider.connection,
      lamports: 1e9,
      recipientPublicKey: rpcRecipientSol,
    });
    let rpc = new TestRpc({
      rpcPubkey: wallet.publicKey,
      rpcRecipientSol: rpcRecipientSol,
      rpcFee: new BN(100_000),
      payer: wallet,
      connection: provider.connection,
      lightWasm: WASM,
    });

    // The light provider is a connection and wallet abstraction.
    // The wallet is used to derive the seed for your compressed keypair with a signature.
    const lightProvider = await LightProvider.init({
      wallet,
      url: RPC_URL,
      rpc,
      confirmConfig,
    });
    const lightUser: User = await User.init({ provider: lightProvider });

    const utxoData = { endSlot: new BN(1), rate: new BN(1) };
    // Issue is that we add + OutUtxo to utxoName
    // -> need to change that in macro circom
    // add function which iterates over all accounts trying to match the discriminator
    const outputUtxoSol = createProgramOutUtxo({
      lightWasm: WASM,
      assets: [SystemProgram.programId],
      amounts: [new BN(1_000_000)],
      data: utxoData,
      ownerIdl: IDL,
      owner: verifierProgramId,
      type: "utxo",
      dataHash: createDataHashWithDefaultHashingSchema(utxoData, WASM),
    });
    const testInputsCompress = {
      utxo: outputUtxoSol,
      action: Action.COMPRESS,
    };

    const storeProgramUtxoResult = await compressProgramUtxo({
      account: lightUser.account,
      appUtxo: testInputsCompress.utxo,
      provider: lightProvider,
    });
    console.log("storeProgramUtxoResult: ", storeProgramUtxoResult);
    const programUtxoBalance: Map<string, ProgramUtxoBalance> =
      await lightUser.syncStorage(IDL);
    const compressedUtxoCommitmentHash = testInputsCompress.utxo.hash;
    console.log(
      "compressedUtxoCommitmentHash: ",
      compressedUtxoCommitmentHash,
      "\n as string:",
      compressedUtxoCommitmentHash.toString()
    );

    console.log(
      "programUtxoBalance get:: ",
      programUtxoBalance
        .get(verifierProgramId.toBase58())
        .tokenBalances.get(testInputsCompress.utxo.assets[0].toBase58())
    );
    const inputUtxo = programUtxoBalance
      .get(verifierProgramId.toBase58())
      .tokenBalances.get(testInputsCompress.utxo.assets[0].toBase58())
      .utxos.get(compressedUtxoCommitmentHash.toString());
    console.log("inputUtxo", inputUtxo);
    compareOutUtxos(inputUtxo!, testInputsCompress.utxo);
    const circuitPath = path.join(
      "build-circuit/streaming-payments/streamingPayments"
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
      checkedInUtxos: [{ type: "streamInUtxo", utxo: inputUtxo }],
    };

    const compressedTransaction = await createTransaction({
      inputUtxos: [inputUtxo],
      merkleTreeSetPubkey: MERKLE_TREE_SET,
      rpcPublicKey: rpc.accounts.rpcPubkey,
      lightWasm: WASM,
      rpcFee: rpc.rpcFee,
      pspId: verifierProgramId,
      systemPspId: lightPsp4in4outAppStorageId,
      account: lightUser.account,
    });
    // createProofInputsAndProve
    const { root, index: rootIndex } =
      (await rpc.getMerkleRoot(MERKLE_TREE_SET))!;
    const proofInputs = createProofInputs({
      lightWasm: WASM,
      transaction: compressedTransaction,
      pspTransaction: pspTransactionInput,
      account: lightUser.account,
      root,
    });

    const systemProof = await getSystemProof({
      account: lightUser.account,
      systemProofInputs: proofInputs,
      verifierIdl: IDL_LIGHT_PSP4IN4OUT_APP_STORAGE,
      inputUtxos: compressedTransaction.private.inputUtxos,
    });

    const completePspProofInputs = setUndefinedPspCircuitInputsToZero(
      proofInputs,
      IDL,
      pspTransactionInput.circuitName
    );

    const pspProof = await lightUser.account.getProofInternal({
      firstPath: pspTransactionInput.path,
      verifierIdl: pspTransactionInput.verifierIdl,
      proofInput: completePspProofInputs,
      inputUtxos: [inputUtxo],
    });

    const solanaTransactionInputs: SolanaTransactionInputs = {
      action: Action.TRANSFER,
      merkleTreeSet: MERKLE_TREE_SET,
      systemProof,
      pspProof,
      publicTransactionVariables: compressedTransaction.public,
      pspTransactionInput,
      rpcRecipientSol: rpc.accounts.rpcRecipientSol,
      systemPspIdl: IDL_LIGHT_PSP4IN4OUT_APP_STORAGE,
      rootIndex,
    };

    await sendAndConfirmCompressedTransaction({
      solanaTransactionInputs,
      provider: lightProvider,
    });
  }
});
