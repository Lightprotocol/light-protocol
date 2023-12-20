import { PublicKey, Keypair as SolanaKeypair } from "@solana/web3.js";

import {
  initLookUpTableFromFile,
  ADMIN_AUTH_KEYPAIR,
  createTestAccounts,
  confirmConfig,
  TestRelayer,
  Action,
  Provider,
  User,
  MINT,
  airdropShieldedSol,
  airdropShieldedMINTSpl,
  RELAYER_FEE,
  airdropSol,
} from "../../src";
import { WasmHasher } from "@lightprotocol/account.rs";
import {
  performShielding,
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

    environmentConfig.hasher = await WasmHasher.getInstance();
    // this keypair is used to derive the shielded account seed from the light message signature
    environmentConfig.providerSolanaKeypair = ADMIN_AUTH_KEYPAIR;
    const relayerRecipientSol = SolanaKeypair.generate().publicKey;

    await anchorProvider.connection.requestAirdrop(relayerRecipientSol, 2e9);

    environmentConfig.relayer = new TestRelayer({
      relayerPubkey: ADMIN_AUTH_KEYPAIR.publicKey,
      relayerRecipientSol,
      relayerFee: RELAYER_FEE,
      payer: ADMIN_AUTH_KEYPAIR,
      connection: anchorProvider.connection,
      hasher: environmentConfig.hasher,
    });
    provider = await Provider.init({
      wallet: environmentConfig.providerSolanaKeypair!,
      relayer: environmentConfig.relayer,
      confirmConfig,
    });
    await airdropSol({
      recipientPublicKey: userKeypair.publicKey,
      lamports: 1000e9,
      connection: anchorProvider.connection,
    });
  });

  it("Merge all sol & spl (no existing utxo)", async () => {
    const testInputsShieldSol = {
      amountSpl: 0,
      amountSol: 15,
      token: "SOL",
      type: Action.SHIELD,
      expectedUtxoHistoryLength: 1,
      recipientAccount: userKeypair,
      mergedUtxo: false,
      shieldToRecipient: true,
      recipientSeed,
    };

    await performShielding({
      numberOfShields: 1,
      testInputs: testInputsShieldSol,
      environmentConfig,
    });

    const testInputsShieldSpl = {
      amountSpl: 20,
      token: "USDC",
      type: Action.SHIELD,
      expectedUtxoHistoryLength: 1,
      expectedSpentUtxosLength: 0,
      shieldToRecipient: true,
      recipientSeed,
    };

    await performShielding({
      numberOfShields: 1,
      testInputs: testInputsShieldSpl,
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
    const testInputsShield = {
      amountSpl: 20,
      token: "USDC",
      type: Action.SHIELD,
      expectedUtxoHistoryLength: 1,
      expectedSpentUtxosLength: 0,
      shieldToRecipient: true,
      recipientSeed,
    };

    await performShielding({
      numberOfShields: 2,
      testInputs: testInputsShield,
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
      relayer: environmentConfig.relayer,
      confirmConfig,
    });
    await airdropShieldedMINTSpl({
      seed: recipientSeed,
      amount: 11,
      provider,
    });
    // shield SPL to recipient
    const testInputsShield = {
      amountSpl: 20,
      token: "USDC",
      type: Action.SHIELD,
      expectedUtxoHistoryLength: 1,
      expectedSpentUtxosLength: 0,
      shieldToRecipient: true,
      recipientSeed,
    };

    await performShielding({
      numberOfShields: 2,
      testInputs: testInputsShield,
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
    await airdropShieldedSol({
      seed: recipientSeed,
      amount: 15,
      provider,
    });

    const testInputsShield = {
      amountSol: 20,
      token: "SOL",
      type: Action.SHIELD,
      expectedUtxoHistoryLength: 1,
      expectedSpentUtxosLength: 0,
      shieldToRecipient: true,
      recipientSeed,
    };

    await performShielding({
      numberOfShields: 2,
      testInputs: testInputsShield,
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
      relayer: environmentConfig.relayer,
      confirmConfig,
    });
    await airdropShieldedSol({
      seed: recipientSeed,
      amount: 1,
      provider,
    });
    const testInputsShield = {
      amountSpl: 0,
      amountSol: 20,
      token: "SOL",
      type: Action.SHIELD,
      expectedUtxoHistoryLength: 1,
      expectedSpentUtxosLength: 0,
      shieldToRecipient: true,
      recipientSeed,
    };

    await performShielding({
      numberOfShields: 2,
      testInputs: testInputsShield,
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
