import { AnchorProvider, BN } from "@coral-xyz/anchor";
import { Keypair, PublicKey, SystemProgram } from "@solana/web3.js";
import chai, { assert, use } from "chai";
import chaiHttp from "chai-http";
import express from "express";

import {
  Account,
  DEFAULT_ZERO,
  FEE_ASSET,
  MerkleTree,
  MERKLE_TREE_HEIGHT,
  TRANSACTION_MERKLE_TREE_KEY,
  MINT,
  Provider,
  TOKEN_REGISTRY,
  Utxo,
  airdropSol,
  confirmConfig,
  User,
  TestRelayer,
  LOOK_UP_TABLE,
} from "@lightprotocol/zk.js";
import sinon from "sinon";
let circomlibjs = require("circomlibjs");
import {
  initMerkleTree,
  initLookupTable,
  updateMerkleTree,
  getIndexedTransactions,
  handleRelayRequest,
  runIndexer,
} from "../src/services";
import { testSetup } from "../src/setup";
import { getKeyPairFromEnv, getLightProvider } from "../src/utils/provider";
import { getTransactions } from "../src/db/redis";
import { DB_VERSION } from "../src/config";
import { Relayer } from "@lightprotocol/zk.js/lib/relayer";
const bs58 = require("bs58");

chai.use(chaiHttp);
const expect = chai.expect;
const app = express();
app.use(express.json());
app.use(express.urlencoded({ extended: false }));

// Use sinon to create a stub for the middleware
const addCorsHeadersStub = sinon
  .stub()
  .callsFake((req: any, res: any, next: any) => next());
app.use(addCorsHeadersStub);

app.post("/updatemerkletree", updateMerkleTree);
app.get("/merkletree", initMerkleTree);
app.get("/lookuptable", initLookupTable);
app.post("/relayTransaction", handleRelayRequest);
app.get("/indexedTransactions", getIndexedTransactions);

describe("Indexer tests", () => {
  let poseidon;
  let depositAmount = 20_000;
  let depositFeeAmount = 10_000;
  let seed32 = bs58.encode(new Uint8Array(32).fill(1));
  let previousMerkleRoot =
    "15800883723037093133305280672853871715176051618981698111580373208012928757479";
  let userKeypair = getKeyPairFromEnv("KEY_PAIR");
  console.log("setting-up test relayer...");
  const testRelayer = new TestRelayer(
    userKeypair.publicKey,
    LOOK_UP_TABLE,
    userKeypair.publicKey,
    new BN(100_000),
  );
  process.env.ANCHOR_WALLET = process.env.HOME + "/.config/solana/id.json";
  process.env.ANCHOR_PROVIDER_URL = "http://127.0.0.1:8899";
  const provider = AnchorProvider.local("http://127.0.0.1:8899", confirmConfig);
  before(async () => {
    poseidon = await circomlibjs.buildPoseidonOpt();
    await testSetup();
    await airdropSol({
      provider,
      lamports: 10_000_000_000,
      recipientPublicKey: getKeyPairFromEnv("KEY_PAIR").publicKey,
    });
  });

  // it should index txs properly
  it("should check for and index existing txs", async () => {
    await runIndexer(2);
  });

  it.only("index correctly", async () => {
    let relayer = new Relayer(userKeypair.publicKey, LOOK_UP_TABLE);
    let indexedTransactions = await relayer.getIndexedTransactions(
      provider.connection,
    );
    console.log(
      "@indexedTransactioNs : indexedTransactions",
      indexedTransactions,
    );
  });

  // it should index txs properly
  it("should check for and index existing txs properly", async () => {
    let { transactions } = await getTransactions(DB_VERSION);
    console.log("transactions", transactions);
    // check keys all there ?
  });

  it.skip("should shield and update merkle tree", async () => {
    let amount = 15;
    let token = "SOL";

    console.log("setting-up test relayer...");

    const provider = await Provider.init({
      wallet: userKeypair,
      relayer: testRelayer,
    });
    await airdropSol({
      provider: provider.provider!,
      lamports: 1000 * 1e9,
      recipientPublicKey: userKeypair.publicKey,
    });
    await airdropSol({
      provider: provider.provider!,
      lamports: 1000 * 1e9,
      // recipientPublicKey: provider.relayer.accounts.relayerRecipientSol,
      recipientPublicKey: testRelayer.accounts.relayerRecipientSol,
    });

    const user: User = await User.init({ provider });

    const tokenCtx = TOKEN_REGISTRY.get(token);

    const preShieldedBalance = await user.getBalance();
    let solShieldedBalancePre = preShieldedBalance.tokenBalances.get(
      SystemProgram.programId.toBase58(),
    )?.totalBalanceSol;

    console.log("solShieldedBalancePre", solShieldedBalancePre);
    await user.shield({ publicAmountSol: amount, token });

    await user.provider.latestMerkleTree();

    let balance = await user.getBalance();

    let solShieldedBalanceAfter = balance.tokenBalances.get(
      SystemProgram.programId.toBase58(),
    )?.totalBalanceSol;

    assert.equal(
      solShieldedBalanceAfter!.toNumber(),
      solShieldedBalancePre!.toNumber() +
        amount * tokenCtx!.decimals.toNumber(),
      `shielded balance after ${solShieldedBalanceAfter!.toString()} != shield amount ${
        amount * tokenCtx!.decimals.toNumber()
      }`,
    );

    assert.notEqual(
      provider.solMerkleTree!.merkleTree.root().toString(),
      previousMerkleRoot,
    );

    previousMerkleRoot = provider.solMerkleTree!.merkleTree.root().toString();

    assert.equal(provider.solMerkleTree!.merkleTree._layers[0].length, 2);

    assert.equal(
      user.balance.tokenBalances.get(tokenCtx!.mint.toBase58())?.utxos.size,
      1,
    );

    assert.equal(
      provider.solMerkleTree!.merkleTree.indexOf(
        user.balance.tokenBalances
          .get(tokenCtx!.mint.toBase58())
          ?.utxos.values()
          .next()
          .value.getCommitment(poseidon),
      ),
      0,
    );
  });
  it.skip("(user class) unshield SOL", async () => {
    let amount = 1;
    let token = "SOL";
    let recipient = Keypair.generate().publicKey;
    const provider = await Provider.init({
      wallet: userKeypair,
      relayer: testRelayer,
    });
    // get token from registry
    const tokenCtx = TOKEN_REGISTRY.get(token);

    const user: User = await User.init({ provider });
    const preShieldedBalance = await user.getBalance();
    let solBalancePre = preShieldedBalance.tokenBalances.get(
      SystemProgram.programId.toString(),
    )?.totalBalanceSol;
    console.log("@unshield solBalancePre", solBalancePre);

    await user.unshield({
      publicAmountSol: amount,
      token,
      recipient,
    });

    await user.provider.latestMerkleTree();

    let balance = await user.getBalance();

    // assert that the user's sol shielded balance has decreased by fee
    let solBalanceAfter = balance.tokenBalances.get(
      SystemProgram.programId.toString(),
    )?.totalBalanceSol;

    assert.equal(
      solBalanceAfter!.toNumber(),
      solBalancePre!.toNumber() -
        100000 -
        amount * tokenCtx!.decimals.toNumber(),
      `shielded sol balance after ${solBalanceAfter!.toString()} != ${solBalancePre!.toString()} ...unshield amount -fee`,
    );

    assert.notEqual(
      provider.solMerkleTree!.merkleTree.root().toString(),
      previousMerkleRoot,
    );

    assert.equal(
      user.balance.tokenBalances.get(SystemProgram.programId.toString())?.utxos
        .size,
      1,
    );
  });
});
