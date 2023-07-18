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
  sleep,
  airdropShieldedSol,
} from "@lightprotocol/zk.js";
import sinon from "sinon";
let circomlibjs = require("circomlibjs");
import {
  initMerkleTree,
  initLookupTable,
  updateMerkleTree,
  getIndexedTransactions,
  handleRelayRequest,
} from "../src/services";
import { testSetup } from "../src/setup";
import { getKeyPairFromEnv, getLightProvider } from "../src/utils/provider";
const bs58 = require("bs58");
import IORedis from "ioredis";

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

describe("API tests", () => {
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
    userKeypair.publicKey,
    new BN(100_000),
    new BN(100_000),
    userKeypair
  );
  before(async () => {
    process.env.ANCHOR_WALLET = process.env.HOME + "/.config/solana/id.json";
    process.env.ANCHOR_PROVIDER_URL = "http://127.0.0.1:8899";
    const provider = AnchorProvider.local(
      "http://127.0.0.1:8899",
      confirmConfig,
    );
    poseidon = await circomlibjs.buildPoseidonOpt();
    await testSetup();
    await airdropSol({
      provider,
      lamports: 10_000_000_000,
      recipientPublicKey: getKeyPairFromEnv("KEY_PAIR").publicKey,
    });
  });

  // it.only("Should return look up table data", (done) => {
  //   const redisConnection = new IORedis({ maxRetriesPerRequest: null });
  //   console.log("redisConnection", redisConnection);
  // });

  it("Should return Merkle tree data", (done) => {
    chai
      .request(app)
      .get("/merkletree")
      .end((err, res) => {
        expect(res).to.have.status(200);

        const fetchedMerkleTree: MerkleTree = res.body.data.merkleTree;

        const pubkey = new PublicKey(res.body.data.pubkey);

        const merkleTree = new MerkleTree(
          MERKLE_TREE_HEIGHT,
          poseidon,
          fetchedMerkleTree._layers[0],
        );
        let lookUpTable = [FEE_ASSET.toBase58(), MINT.toBase58()];
        const deposit_utxo1 = new Utxo({
          poseidon: poseidon,
          assets: [FEE_ASSET, MINT],
          amounts: [new BN(depositFeeAmount), new BN(depositAmount)],
          account: new Account({ poseidon: poseidon, seed: seed32 }),
          blinding: new BN(new Array(31).fill(1)),
          assetLookupTable: lookUpTable,
          verifierProgramLookupTable: lookUpTable,
        });

        expect(res.body.data.merkleTree).to.exist;
        expect(res.body.data).to.exist;
        assert.equal(merkleTree.levels, MERKLE_TREE_HEIGHT);
        assert.equal(pubkey.toBase58(), TRANSACTION_MERKLE_TREE_KEY.toBase58());
        assert.equal(merkleTree.root().toString(), previousMerkleRoot);
        assert.equal(merkleTree._layers[0].length, 0);
        assert.equal(merkleTree.zeroElement, DEFAULT_ZERO);
        assert.equal(
          merkleTree.indexOf(deposit_utxo1.getCommitment(poseidon)),
          -1,
        );

        done();
      });
  });

  it("Should fail Merkle tree data with post request", (done) => {
    chai
      .request(app)
      .post("/merkletree")
      .end((err, res) => {
        assert.isTrue(
          res.error.message.includes("cannot POST /merkletree (404)"),
        );
        expect(res).to.have.status(404);
        done();
      });
  });

  it("Should fail to update Merkle tree with InvalidNumberOfLeaves", (done) => {
    chai
      .request(app)
      .post("/updatemerkletree")
      .end((err, res) => {
        expect(res).to.have.status(500);
        // TODO: fix error propagation
        // assert.isTrue(
        // res.body.message.includes("Error Message: InvalidNumberOfLeaves."),
        // );
        expect(res.body.status).to.be.equal("error");
        done();
      });
  });
  it.skip("should shield and update merkle tree", async () => {
    const provider = await Provider.init({
      wallet: userKeypair,
    });
   
    const user: User = await User.init({ provider, skipFetchBalance: true});
    const indexedTransactionsPre = await testRelayer.getIndexedTransactions(provider.provider!.connection);
    console.log("testRelayer.indexedTransactions.length ", testRelayer.indexedTransactions.length);
    
    let res = await user.shield({ publicAmountSol: 1, token: "SOL" });
    console.log("res ", res);
    const indexedTransactionsPost = await testRelayer.getIndexedTransactions(provider.provider!.connection);
  });

  it.only("should shield and update merkle tree", async () => {
    let amount = 15;
    let token = "SOL";

    const provider = await Provider.init({
      wallet: userKeypair,
      confirmConfig,
    });
    await airdropSol({
      provider: provider.provider!,
      lamports: 1000 * 1e9,
      recipientPublicKey: userKeypair.publicKey,
    });
    await airdropSol({
      provider: provider.provider!,
      lamports: 1000 * 1e9,
      recipientPublicKey: provider.relayer.accounts.relayerRecipientSol,
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
    let retries = 5;
    while (retries > 0) {
      retries--;
      console.log("retries", retries); 
      if (balance.totalSolBalance.gt(new BN(0)))
        retries = 0;
      balance = await user.getBalance();
      await sleep(2000);
    }

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

  it("Should fail to update Merkle tree", (done) => {
    chai
      .request(app)
      .get("/updatemerkletree")
      .end((err, res) => {
        assert.isTrue(
          res.error.message.includes("cannot GET /updatemerkletree (404)"),
        );
        expect(res).to.have.status(404);
        done();
      });
  });

  it("Should return lookup table data", (done) => {
    chai
      .request(app)
      .get("/lookuptable")
      .end(async (err, res) => {
        const provider = await Provider.init({
          wallet: userKeypair,
        });

        let lookUpTableInfo =
          await provider.provider!.connection.getAccountInfo(
            new PublicKey(res.body.data),
          );

        assert.notEqual(lookUpTableInfo, null);
        expect(new PublicKey(res.body.data).toString()).to.exist;
        expect(res.body.data).to.exist;
        expect(res).to.have.status(200);
        done();
      });
  });

  it("Should fail to return lookup table data", (done) => {
    chai
      .request(app)
      .post("/lookuptable")
      .end((err, res) => {
        assert.isTrue(
          res.error.message.includes("cannot POST /lookuptable (404)"),
        );
        expect(res).to.have.status(404);
        done();
      });
  });

  it("transfer SPL token", async () => {
    console.log("initializing light provider recipient...");
    let amount = 1;
    let token = "SOL";

    // REINIT SAME USER AS SHIELD
    const provider = await Provider.init({
      wallet: userKeypair,
    });
    const user: User = await User.init({ provider });

    // INIT RECIPIENT
    let recipientKeypair = Keypair.generate();
    let recipient = recipientKeypair.publicKey;
    await airdropSol({
      provider: provider.provider!,
      lamports: 1000 * 1e9,
      recipientPublicKey: recipientKeypair.publicKey,
    });
    const lightProviderRecipient = await Provider.init({
      wallet: recipientKeypair,
    });
    const testRecipient: User = await User.init({
      provider: lightProviderRecipient,
    });

    const response = await user.transfer({
      amountSol: "1",
      token: "SOL",
      recipient: testRecipient.account.getPublicKey(),
    });
    console.log("TRANSFER response", response);
    // check balance of response
  });
  // TODO: add a shield... before, add a transfer too tho, => assert job queeing functioning etc
  it.only("(user class) unshield SOL", async () => {
    
    let amount = 1;
    let token = "SOL";
    let recipient = Keypair.generate().publicKey;
    const provider = await Provider.init({
      wallet: userKeypair,
    });
    console.log("provider.relayer ", provider.relayer);

    // get token from registry
    const tokenCtx = TOKEN_REGISTRY.get(token);

    const user: User = await User.init({ provider });

    const preShieldedBalance = await user.getBalance();
    let solBalancePre = preShieldedBalance.tokenBalances.get(
      SystemProgram.programId.toString(),
    )?.totalBalanceSol;
    console.log("@unshield solBalancePre", solBalancePre);
    console.time("unshield");
    await user.unshield({
      publicAmountSol: amount,
      token,
      recipient,
    });
    console.timeEnd("unshield");

    let balance = await user.getBalance();
    let retries = 5;
    while (retries > 0) {
      retries--;
      console.log("retries", retries); 
      if (balance.totalSolBalance.gt(new BN(0)))
        retries = 0;
      balance = await user.getBalance();
      await sleep(2000);
    }
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

  // TODO: add test for just proper indexing (-> e.g. shields)
  // TODO: add test for stress test load (multiple requests, wrong requests etc)
  it("Should fail transaction with empty instructions", (done) => {
    const instructions = []; // Replace with a valid instruction object
    chai
      .request(app)
      .post("/relayTransaction")
      .send({ instructions })
      .end((err, res) => {
        expect(res).to.have.status(500);
        assert.isTrue(res.body.message.includes("No instructions provided"));
        done();
      });
  });
});
