import * as anchor from "@coral-xyz/anchor";
import { assert } from "chai";
import {
  Utxo,
  createTransaction,
  Provider as LightProvider,
  confirmConfig,
  Action,
  TestRpc,
  User,
  ProgramUtxoBalance,
  airdropSol,
  PspTransactionInput,
  getSystemProof,
  MerkleTreeConfig,
  lightPsp4in4outAppStorageId,
  IDL_LIGHT_PSP4IN4OUT_APP_STORAGE,
  createProofInputs,
  SolanaTransactionInputs,
  sendAndConfirmShieldedTransaction,
  getVerifierProgramId,
  shieldProgramUtxo,
  createProgramOutUtxo,
  createOutUtxo,
} from "@lightprotocol/zk.js";
import { WasmFactory } from "@lightprotocol/account.rs";
import { SystemProgram, PublicKey, Keypair } from "@solana/web3.js";

import { BN } from "@coral-xyz/anchor";
import { IDL } from "../target/types/{{rust-name}}";
const path = require("path");

const verifierProgramId = new PublicKey("{{program-id}}");
let WASM;

const RPC_URL = "http://127.0.0.1:8899";

describe("Test {{project-name}}", () => {
  process.env.ANCHOR_PROVIDER_URL = RPC_URL;
  process.env.ANCHOR_WALLET = process.env.HOME + "/.config/solana/id.json";

  // Configure the client to use the local cluster.
  const provider = anchor.AnchorProvider.local(RPC_URL, confirmConfig);
  anchor.setProvider(provider);

  before(async () => {
    WASM = await WasmFactory.getInstance();
  });

  it("Create and Spend Program Utxo ", async () => {
    const wallet = Keypair.generate();
    await airdropSol({
      connection: provider.connection,
      lamports: 1e10,
      recipientPublicKey: wallet.publicKey,
    });

    let rpc = new TestRpc({
      rpcPubkey: wallet.publicKey,
      rpcRecipientSol: wallet.publicKey,
      rpcFee: new BN(100000),
      payer: wallet,
      connection: provider.connection,
      lightWasm: WASM,
    });

    // The light provider is a connection and wallet abstraction.
    // The wallet is used to derive the seed for your shielded keypair with a signature.
    var lightProvider = await LightProvider.init({
      wallet,
      url: RPC_URL,
      rpc,
      confirmConfig,
    });
    lightProvider.addVerifierProgramPublickeyToLookUpTable(
      getVerifierProgramId(IDL),
    );

    const user: User = await User.init({ provider: lightProvider });

    const outputUtxoSol = createProgramOutUtxo({
      lightWasm: WASM,
      assets: [SystemProgram.programId],
      publicKey: user.account.keypair.publicKey,
      amounts: [new BN(1_000_000)],
      utxoData: { x: new BN(1), y: new BN(2) },
      pspIdl: IDL,
      pspId: verifierProgramId,
      utxoName: "utxo",
    });

    const testInputsShield = {
      utxo: outputUtxoSol,
      action: Action.SHIELD,
    };

    let storeTransaction = await shieldProgramUtxo({
      account: user.account,
      appUtxo: testInputsShield.utxo,
      provider: user.provider,
    });
    console.log("store program utxo transaction hash ", storeTransaction);

    const programUtxoBalance: Map<string, ProgramUtxoBalance> =
      await user.syncStorage(IDL);
    const shieldedUtxoCommitmentHash = testInputsShield.utxo.outUtxo.utxoHash;
    const inputUtxo = programUtxoBalance
      .get(verifierProgramId.toBase58())!
      .tokenBalances.get(testInputsShield.utxo.outUtxo.assets[1].toBase58())!
      .utxos.get(shieldedUtxoCommitmentHash)!;
    assert.equal(inputUtxo.utxoHash, shieldedUtxoCommitmentHash);
    assert.equal(inputUtxo.utxoData.x.toString(), "1");
    assert.equal(inputUtxo.utxoData.y.toString(), "2");

    const circuitPath = path.join(
      `build-circuit/${"{{project-name}}"}/${"{{circom-name-camel-case}}"}`,
    );

    const pspTransactionInput: PspTransactionInput = {
      proofInputs: {
        publicZ: inputUtxo.utxoData.x.add(inputUtxo.utxoData.y),
      },
      path: circuitPath,
      verifierIdl: IDL,
      circuitName: "{{circom-name-camel-case}}",
      checkedInUtxos: [{ utxoName: "inputUtxo", utxo: inputUtxo }],
    };
    const changeAmountSol = inputUtxo.amounts[0].sub(rpc.getRpcFee());

    const changeUtxo = createOutUtxo({
      lightWasm: WASM,
      publicKey: new BN(inputUtxo.publicKey),
      amounts: [changeAmountSol],
      assets: [SystemProgram.programId],
    });
    const inputUtxos = [inputUtxo];
    const outputUtxos = [changeUtxo];
    const shieldedTransaction = await createTransaction({
      inputUtxos,
      outputUtxos,
      transactionMerkleTreePubkey: MerkleTreeConfig.getTransactionMerkleTreePda(
        new BN(0),
      ),
      rpcPublicKey: rpc.accounts.rpcPubkey,
      lightWasm: WASM,
      rpcFee: rpc.getRpcFee(),
      pspId: verifierProgramId,
      systemPspId: lightPsp4in4outAppStorageId,
      account: user.account,
    });

    const { root, index: rootIndex } = (await rpc.getMerkleRoot(
      MerkleTreeConfig.getTransactionMerkleTreePda(),
    ))!;

    const proofInputs = createProofInputs({
      lightWasm: WASM,
      transaction: shieldedTransaction,
      pspTransaction: pspTransactionInput,
      account: user.account,
      root,
    });

    const systemProof = await getSystemProof({
      account: user.account,
      systemProofInputs: proofInputs,
      verifierIdl: IDL_LIGHT_PSP4IN4OUT_APP_STORAGE,
      inputUtxos,
    });

    const pspProof = await user.account.getProofInternal({
      firstPath: pspTransactionInput.path,
      verifierIdl: pspTransactionInput.verifierIdl,
      proofInput: proofInputs,
      inputUtxos,
    });
    const solanaTransactionInputs: SolanaTransactionInputs = {
      action: Action.TRANSFER,
      systemProof,
      pspProof,
      publicTransactionVariables: shieldedTransaction.public,
      pspTransactionInput,
      rpcRecipientSol: rpc.accounts.rpcRecipientSol,
      eventMerkleTree: MerkleTreeConfig.getEventMerkleTreePda(),
      systemPspIdl: IDL_LIGHT_PSP4IN4OUT_APP_STORAGE,
      rootIndex,
    };

    const { txHash } = await sendAndConfirmShieldedTransaction({
      solanaTransactionInputs,
      provider: user.provider,
    });

    console.log("transaction hash ", txHash);
    const utxoSpent = await user.getUtxo(
      inputUtxo.utxoHash,
      true,
      MerkleTreeConfig.getTransactionMerkleTreePda(),
      IDL,
    );
    assert.equal(utxoSpent!.status, "spent");
  });
});
