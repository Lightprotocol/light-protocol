import * as anchor from "@coral-xyz/anchor";
import { assert } from "chai";
import {
  Utxo,
  TransactionParameters,
  Provider as LightProvider,
  confirmConfig,
  Action,
  TestRelayer,
  User,
  ProgramUtxoBalance,
  airdropSol,
  PspTransactionInput,
  ConfirmOptions,
  getSystemProof,
  MerkleTreeConfig,
  getVerifierStatePda,
  IDL_LIGHT_PSP4IN4OUT_APP_STORAGE,
  createProofInputs,
  SolanaTransactionInputs,
  sendAndConfirmShieldedTransaction
} from "@lightprotocol/zk.js";
import { Hasher, WasmHasher } from "@lightprotocol/account.rs";
import {
  SystemProgram,
  PublicKey,
  Keypair,
} from "@solana/web3.js";


import { BN } from "@coral-xyz/anchor";
import { IDL } from "../target/types/{{rust-name}}";
const path = require("path");

const verifierProgramId = new PublicKey(
  "{{program-id}}",
);
let HASHER;

const RPC_URL = "http://127.0.0.1:8899";

describe("Test {{project-name}}", () => {
  process.env.ANCHOR_PROVIDER_URL = RPC_URL;
  process.env.ANCHOR_WALLET = process.env.HOME + "/.config/solana/id.json";

  // Configure the client to use the local cluster.
  const provider = anchor.AnchorProvider.local(RPC_URL, confirmConfig);
  anchor.setProvider(provider);

  before(async () => {
    HASHER = await WasmHasher.getInstance();
  });


  it("Create and Spend Program Utxo ", async () => {
    const wallet = Keypair.generate();
    await airdropSol({
      connection: provider.connection,
      lamports: 1e10,
      recipientPublicKey: wallet.publicKey,
    });

    let relayer = new TestRelayer({
      relayerPubkey: wallet.publicKey,
      relayerRecipientSol:  wallet.publicKey,
      relayerFee: new BN(100000),
      payer: wallet
    });

    // The light provider is a connection and wallet abstraction.
    // The wallet is used to derive the seed for your shielded keypair with a signature.
    var lightProvider = await LightProvider.init({ wallet, url: RPC_URL, relayer, confirmConfig });
    lightProvider.addVerifierProgramPublickeyToLookUpTable(TransactionParameters.getVerifierProgramId(IDL));

    const user: User = await User.init({ provider: lightProvider });

    const outputUtxoSol = new Utxo({
      hasher: HASHER,
      assets: [SystemProgram.programId],
      publicKey: user.account.pubkey,
      amounts: [new BN(1_000_000)],
      appData: { x: new BN(1), y: new BN(1) },
      appDataIdl: IDL,
      verifierAddress: verifierProgramId,
      assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
    });

    const testInputsShield = {
      utxo: outputUtxoSol,
      action: Action.SHIELD,
    }

    let storeTransaction = await user.storeAppUtxo({
      appUtxo: testInputsShield.utxo,
      action: testInputsShield.action,
    });
    console.log("store program utxo transaction hash ", storeTransaction.txHash);

    const programUtxoBalance: Map<string, ProgramUtxoBalance> =
      await user.syncStorage(IDL);
    const shieldedUtxoCommitmentHash =
      testInputsShield.utxo.getCommitment(HASHER);
    const inputUtxo = programUtxoBalance
      .get(verifierProgramId.toBase58())
      .tokenBalances.get(testInputsShield.utxo.assets[1].toBase58())
      .utxos.get(shieldedUtxoCommitmentHash);

    Utxo.equal(HASHER, inputUtxo, testInputsShield.utxo, true);

    const circuitPath = path.join(`build-circuit/${"{{project-name}}"}/${"{{circom-name-camel-case}}"}`);

    const pspTransactionInput: PspTransactionInput = {
      proofInputs: {
        publicZ: inputUtxo.appData.x.add(inputUtxo.appData.y),
      },
      path: circuitPath,
      verifierIdl: IDL,
      circuitName: "{{circom-name-camel-case}}",
      checkedInUtxos: [{ utxoName: "inputUtxo", utxo: inputUtxo }],
    };
    const changeAmountSol = inputUtxo.amounts[0]
      .sub(relayer.relayerFee);
    const changeUtxo = new Utxo({
      hasher: HASHER,
      publicKey: inputUtxo.publicKey,
      assetLookupTable: user.provider.lookUpTables.assetLookupTable,
      amounts: [changeAmountSol],
      assets: [SystemProgram.programId],
    });
    const inputUtxos = [inputUtxo];
    const outputUtxos = [changeUtxo];

    const txParams = new TransactionParameters({
      inputUtxos,
      outputUtxos,
      transactionMerkleTreePubkey: MerkleTreeConfig.getTransactionMerkleTreePda(
        new BN(0),
      ),
      eventMerkleTreePubkey: MerkleTreeConfig.getEventMerkleTreePda(new BN(0)),
      action: Action.TRANSFER,
      hasher: HASHER,
      relayer,
      verifierIdl: IDL_LIGHT_PSP4IN4OUT_APP_STORAGE,
      account: user.account,
      verifierState: getVerifierStatePda(
        verifierProgramId,
        relayer.accounts.relayerPubkey,
      ),
    });

    await txParams.getTxIntegrityHash(HASHER);

    const proofInputs = createProofInputs({
      hasher: HASHER,
      transaction: txParams,
      pspTransaction: pspTransactionInput,
      account: user.account,
      solMerkleTree: user.provider.solMerkleTree,
    });

    const systemProof = await getSystemProof({
      account: user.account,
      transaction: txParams,
      systemProofInputs: proofInputs,
    });

    const pspProof = await user.account.getProofInternal(
      pspTransactionInput.path,
      pspTransactionInput,
      proofInputs,
      false,
    );
    const solanaTransactionInputs: SolanaTransactionInputs = {
      systemProof,
      pspProof,
      transaction: txParams,
      pspTransactionInput,
    };

    const {txHash} = await sendAndConfirmShieldedTransaction({
      solanaTransactionInputs,
      provider: user.provider,
      confirmOptions: ConfirmOptions.spendable,
    });

    console.log("transaction hash ", txHash);
    const utxoSpent = await user.getUtxo(inputUtxo.getCommitment(HASHER), true, IDL);
    assert.equal(utxoSpent.status, "spent");
    Utxo.equal(HASHER, utxoSpent.utxo, inputUtxo, true);
  });
});
