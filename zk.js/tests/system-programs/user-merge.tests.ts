import { PublicKey, Keypair as SolanaKeypair } from "@solana/web3.js";

import {
  initLookUpTableFromFile,
  ADMIN_AUTH_KEYPAIR,
  createTestAccounts,
  confirmConfig,
  TestRpc,
  Action,
  Provider,
  User,
  MINT,
  airdropCompressedSol,
  airdropCompressedMINTSpl,
  RPC_FEE,
  airdropSol,
} from "../../src";
import { WasmFactory } from "@lightprotocol/account.rs";
import {
  performCompressing,
  EnvironmentConfig,
  performMergeAll,
  performMergeUtxos,
} from "../test-utils/user-utils";

import { AnchorProvider, setProvider } from "@coral-xyz/anchor";
import { bs58 } from "@coral-xyz/anchor/dist/cjs/utils/bytes";

const recipientSeed = bs58.encode(new Uint8Array(32).fill(7));
let provider: Provider;
describe("Test User merge 1 sol utxo and one spl utxo in sequence ", () => {
  // Configure the client to use the local cluster.
  process.env.ANCHOR_WALLET = process.env.HOME + "/.config/solana/id.json";
  process.env.ANCHOR_PROVIDER_URL = "http://127.0.0.1:8899";

  const anchorProvider = AnchorProvider.local(
    "http://127.0.0.1:8899",
    confirmConfig,
  );
  setProvider(anchorProvider);

  const userKeypair = ADMIN_AUTH_KEYPAIR;
  const environmentConfig: EnvironmentConfig = {};

  before("init test setup Merkle tree lookup table etc ", async () => {
    await createTestAccounts(anchorProvider.connection);
    environmentConfig.lookUpTable =
      await initLookUpTableFromFile(anchorProvider);

    environmentConfig.lightWasm = await WasmFactory.getInstance();
    // this keypair is used to derive the compressed account seed from the light message signature
    environmentConfig.providerSolanaKeypair = ADMIN_AUTH_KEYPAIR;
    const rpcRecipientSol = SolanaKeypair.generate().publicKey;

    await anchorProvider.connection.requestAirdrop(rpcRecipientSol, 2e9);

    environmentConfig.rpc = new TestRpc({
      rpcPubkey: ADMIN_AUTH_KEYPAIR.publicKey,
      rpcRecipientSol,
      rpcFee: RPC_FEE,
      payer: ADMIN_AUTH_KEYPAIR,
      connection: anchorProvider.connection,
      lightWasm: environmentConfig.lightWasm,
    });
    provider = await Provider.init({
      wallet: environmentConfig.providerSolanaKeypair!,
      rpc: environmentConfig.rpc,
      confirmConfig,
    });
    await airdropSol({
      recipientPublicKey: userKeypair.publicKey,
      lamports: 1000e9,
      connection: anchorProvider.connection,
    });
  });

  it("Merge all sol & spl (no existing utxo)", async () => {
    const testInputsCompressSol = {
      amountSpl: 0,
      amountSol: 15,
      token: "SOL",
      type: Action.COMPRESS,
      expectedUtxoHistoryLength: 1,
      recipientAccount: userKeypair,
      mergedUtxo: false,
      compressToRecipient: true,
      recipientSeed,
    };

    await performCompressing({
      numberOfCompressions: 1,
      testInputs: testInputsCompressSol,
      environmentConfig,
    });

    const testInputsCompressSpl = {
      amountSpl: 20,
      token: "USDC",
      type: Action.COMPRESS,
      expectedUtxoHistoryLength: 1,
      expectedSpentUtxosLength: 0,
      compressToRecipient: true,
      recipientSeed,
    };

    await performCompressing({
      numberOfCompressions: 1,
      testInputs: testInputsCompressSpl,
      environmentConfig,
    });

    const testInputsMergeAllSol = {
      token: "SOL",
      type: Action.TRANSFER,
      expectedUtxoHistoryLength: 1,
      expectedSpentUtxosLength: 0,
      recipientSeed,
    };

    await performMergeAll({
      environmentConfig,
      testInputs: testInputsMergeAllSol,
    });

    const testInputsMergeAllSpl = {
      type: Action.TRANSFER,
      token: "USDC",
      expectedUtxoHistoryLength: 1,
      expectedSpentUtxosLength: 0,
      recipientSeed,
    };

    await performMergeAll({
      environmentConfig,
      testInputs: testInputsMergeAllSpl,
    });
  });

  it("Merge all spl (existing utxos)", async () => {
    const testInputsCompress = {
      amountSpl: 20,
      token: "USDC",
      type: Action.COMPRESS,
      expectedUtxoHistoryLength: 1,
      expectedSpentUtxosLength: 0,
      compressToRecipient: true,
      recipientSeed,
    };

    await performCompressing({
      numberOfCompressions: 2,
      testInputs: testInputsCompress,
      environmentConfig,
    });

    const testInputs = {
      type: Action.TRANSFER,
      token: "USDC",
      expectedUtxoHistoryLength: 1,
      expectedSpentUtxosLength: 0,
      recipientSeed,
    };
    await performMergeAll({
      environmentConfig,
      testInputs,
    });
  });

  it("Merge one spl (existing utxos)", async () => {
    const provider = await Provider.init({
      wallet: userKeypair,
      rpc: environmentConfig.rpc,
      confirmConfig,
    });
    await airdropCompressedMINTSpl({
      seed: recipientSeed,
      amount: 11,
      provider,
    });
    // compress SPL to recipient
    const testInputsCompress = {
      amountSpl: 20,
      token: "USDC",
      type: Action.COMPRESS,
      expectedUtxoHistoryLength: 1,
      expectedSpentUtxosLength: 0,
      compressToRecipient: true,
      recipientSeed,
    };

    await performCompressing({
      numberOfCompressions: 2,
      testInputs: testInputsCompress,
      environmentConfig,
    });

    const userSender: User = await User.init({
      provider,
      seed: recipientSeed,
    });

    const utxoCommitment: string = (
      await userSender.getUtxoInbox()
    ).tokenBalances
      .get(MINT.toBase58())!
      .utxos.keys()
      .next().value;

    const testInputs = {
      type: Action.TRANSFER,
      token: "USDC",
      expectedUtxoHistoryLength: 1,
      expectedSpentUtxosLength: 0,
      utxoCommitments: [utxoCommitment],
      recipientSeed,
    };

    await performMergeUtxos({
      testInputs,
      environmentConfig,
    });
  });

  it("Merge all sol (existing utxos)", async () => {
    await airdropCompressedSol({
      seed: recipientSeed,
      amount: 15,
      provider,
    });

    const testInputsCompress = {
      amountSol: 20,
      token: "SOL",
      type: Action.COMPRESS,
      expectedUtxoHistoryLength: 1,
      expectedSpentUtxosLength: 0,
      compressToRecipient: true,
      recipientSeed,
    };

    await performCompressing({
      numberOfCompressions: 2,
      testInputs: testInputsCompress,
      environmentConfig,
    });

    const testInputs = {
      type: Action.TRANSFER,
      token: "SOL",
      expectedUtxoHistoryLength: 1,
      expectedSpentUtxosLength: 0,
      recipientSeed,
    };

    await performMergeAll({
      environmentConfig,
      testInputs,
    });
  });

  it("Merge one sol (existing utxos)", async () => {
    const provider = await Provider.init({
      wallet: userKeypair,
      rpc: environmentConfig.rpc,
      confirmConfig,
    });
    await airdropCompressedSol({
      seed: recipientSeed,
      amount: 1,
      provider,
    });
    const testInputsCompress = {
      amountSpl: 0,
      amountSol: 20,
      token: "SOL",
      type: Action.COMPRESS,
      expectedUtxoHistoryLength: 1,
      expectedSpentUtxosLength: 0,
      compressToRecipient: true,
      recipientSeed,
    };

    await performCompressing({
      numberOfCompressions: 2,
      testInputs: testInputsCompress,
      environmentConfig,
    });

    const userSender: User = await User.init({
      provider,
      seed: recipientSeed,
    });

    const utxoCommitment: string = (
      await userSender.getUtxoInbox()
    ).tokenBalances
      .get(PublicKey.default.toBase58())!
      .utxos.keys()
      .next().value;

    const testInputs = {
      type: Action.TRANSFER,
      token: "SOL",
      expectedUtxoHistoryLength: 1,
      expectedSpentUtxosLength: 0,
      utxoCommitments: [utxoCommitment],
      recipientSeed,
    };

    await performMergeUtxos({
      testInputs,
      environmentConfig,
    });
  });
});
