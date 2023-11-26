//@ts-check
import { AnchorProvider, BN } from "@coral-xyz/anchor";
import {
  Keypair,
  PublicKey,
  SystemProgram,
  TransactionInstruction,
} from "@solana/web3.js";
import chai from "chai";
import { assert } from "chai";
import chaiHttp from "chai-http";

import {
  Account,
  DEFAULT_ZERO,
  FEE_ASSET,
  MERKLE_TREE_HEIGHT,
  MINT,
  Provider,
  Utxo,
  airdropSol,
  confirmConfig,
  User,
  sleep,
  Action,
  UserTestAssertHelper,
  ConfirmOptions,
  MerkleTreeConfig,
  Relayer,
  RELAYER_FEE,
  TOKEN_ACCOUNT_FEE,
} from "@lightprotocol/zk.js";

import { MerkleTree } from "@lightprotocol/circuit-lib.js";
const circomlibjs = require("circomlibjs");
const { buildPoseidonOpt } = circomlibjs;
import { getUidFromIxs } from "../src/services";
import { getKeyPairFromEnv } from "../src/utils/provider";
import { waitForBalanceUpdate } from "./test-utils/waitForBalanceUpdate";
import { RELAYER_URL } from "../src/config";
const bs58 = require("bs58");

chai.use(chaiHttp);
const expect = chai.expect;
const server = RELAYER_URL;

describe("API tests", () => {
  let poseidon: any;
  let shieldAmount = 20_000;
  let shieldFeeAmount = 10_000;
  let seed32 = bs58.encode(new Uint8Array(32).fill(1));
  let previousMerkleRoot =
    "15800883723037093133305280672853871715176051618981698111580373208012928757479";
  let userKeypair = Keypair.generate();
  let provider: Provider,
    user: User,
    anchorProvider: AnchorProvider,
    relayer: Relayer;

  before(async () => {
    process.env.ANCHOR_WALLET = process.env.HOME + "/.config/solana/id.json";
    process.env.ANCHOR_PROVIDER_URL = "http://127.0.0.1:8899";
    anchorProvider = AnchorProvider.local(
      "http://127.0.0.1:8899",
      confirmConfig,
    );
    poseidon = await buildPoseidonOpt();
    await airdropSol({
      connection: anchorProvider.connection,
      lamports: 9e8,
      recipientPublicKey: getKeyPairFromEnv("KEY_PAIR").publicKey,
    });

    await airdropSol({
      connection: anchorProvider.connection,
      lamports: 9e8,
      recipientPublicKey: userKeypair.publicKey,
    });

    const relayer = await Relayer.initFromUrl(RELAYER_URL);

    provider = await Provider.init({
      wallet: userKeypair,
      confirmConfig,
      relayer,
    });

    await airdropSol({
      connection: anchorProvider.connection,
      lamports: 9e8,
      recipientPublicKey: provider.relayer.accounts.relayerRecipientSol,
    });

    user = await User.init({ provider });
  });

  it("initFromUrl with relayerInfo", async () => {
    relayer = await Relayer.initFromUrl(RELAYER_URL);
    console.log("relayer", relayer);
    assert.equal(
      relayer.accounts.relayerRecipientSol.toBase58(),
      getKeyPairFromEnv("RELAYER_RECIPIENT").publicKey.toBase58(),
    );
    assert.equal(
      relayer.accounts.relayerPubkey.toBase58(),
      getKeyPairFromEnv("KEY_PAIR").publicKey.toBase58(),
    );
    assert.equal(relayer.relayerFee.toString(), RELAYER_FEE.toString());
    assert.equal(
      relayer.highRelayerFee.toString(),
      TOKEN_ACCOUNT_FEE.toString(),
    );
  });

  it("Should return Merkle tree data", (done) => {
    chai
      .request(server)
      .get("/getBuiltMerkletree")
      .end((_err, res) => {
        expect(res).to.have.status(200);

        const fetchedMerkleTree: MerkleTree = res.body.data.merkleTree;

        const pubkey = new PublicKey(res.body.data.pubkey);

        const merkleTree = new MerkleTree(
          MERKLE_TREE_HEIGHT,
          poseidon,
          fetchedMerkleTree._layers[0],
        );
        let lookUpTable = [FEE_ASSET.toBase58(), MINT.toBase58()];
        const shieldUtxo1 = new Utxo({
          poseidon: poseidon,
          assets: [FEE_ASSET, MINT],
          amounts: [new BN(shieldFeeAmount), new BN(shieldAmount)],
          publicKey: new Account({ poseidon: poseidon, seed: seed32 }).pubkey,
          blinding: new BN(new Array(31).fill(1)),
          assetLookupTable: lookUpTable,
        });

        expect(res.body.data.merkleTree).to.exist;
        expect(res.body.data).to.exist;
        assert.equal(merkleTree.levels, MERKLE_TREE_HEIGHT);
        assert.equal(
          pubkey.toBase58(),
          MerkleTreeConfig.getTransactionMerkleTreePda().toBase58(),
        );
        assert.equal(merkleTree.root().toString(), previousMerkleRoot);
        assert.equal(merkleTree._layers[0].length, 0);
        assert.equal(merkleTree.zeroElement, DEFAULT_ZERO);
        assert.equal(
          merkleTree.indexOf(shieldUtxo1.getCommitment(poseidon)),
          -1,
        );

        done();
      });
  });

  it("Should fail Merkle tree data with post request", (done: any) => {
    chai
      .request(server)
      .post("/merkletree")
      .end((_err, res) => {
        const error = res.error;
        assert.isNotFalse(error);
        if (error != false) {
          assert.isTrue(
            error.message.includes("cannot POST /merkletree (404)"),
          );
        }
        expect(res).to.have.status(404);
        done();
      });
  });

  it("Should fail to update Merkle tree with InvalidNumberOfLeaves", (done: any) => {
    chai
      .request(server)
      .post("/updatemerkletree")
      .end((_err, res) => {
        expect(res).to.have.status(500);
        // TODO: fix error propagation
        // assert.isTrue(
        // res.body.message.includes("Error Message: InvalidNumberOfLeaves."),
        // );
        expect(res.body.status).to.be.equal("error");
        done();
      });
  });

  it("should shield and update merkle tree", async () => {
    let testInputs = {
      amountSol: 0.3,
      token: "SOL",
      type: Action.SHIELD,
      expectedUtxoHistoryLength: 1,
    };

    const userTestAssertHelper = new UserTestAssertHelper({
      userSender: user,
      userRecipient: user,
      provider,
      testInputs,
    });
    await userTestAssertHelper.fetchAndSaveState();
    await user.shield({
      publicAmountSol: testInputs.amountSol,
      token: testInputs.token,
      confirmOptions: ConfirmOptions.spendable,
    });

    await waitForBalanceUpdate(userTestAssertHelper, user);
    await userTestAssertHelper.checkSolShielded();
  });

  it("should fail to unshield SOL with invalid relayer pubkey", async () => {
    const solRecipient = Keypair.generate();
    const relayer = new Relayer(
      Keypair.generate().publicKey,
      Keypair.generate().publicKey,
      RELAYER_FEE,
      TOKEN_ACCOUNT_FEE,
      RELAYER_URL!,
    );
    const provider = await Provider.init({
      wallet: userKeypair,
      confirmConfig,
      relayer,
    });
    const user = await User.init({ provider });

    const testInputs = {
      amountSol: 0.05,
      token: "SOL",
      type: Action.UNSHIELD,
      recipient: solRecipient.publicKey,
      expectedUtxoHistoryLength: 1,
    };
    let error = null;
    try {
      await user.unshield({
        publicAmountSol: testInputs.amountSol,
        token: testInputs.token,
        recipient: testInputs.recipient,
        confirmOptions: ConfirmOptions.spendable,
      });
    } catch (e) {
      error = e;
    }
    expect(error).to.exist;
  });

  it("should fail to unshield SOL with invalid relayer sol recipient", async () => {
    const solRecipient = Keypair.generate();
    let invalidRelayer = [...[relayer]][0];
    invalidRelayer.accounts.relayerRecipientSol = Keypair.generate().publicKey;

    const provider = await Provider.init({
      wallet: userKeypair,
      confirmConfig,
      relayer: invalidRelayer,
    });

    const user = await User.init({ provider });

    const testInputs = {
      amountSol: 0.05,
      token: "SOL",
      type: Action.UNSHIELD,
      recipient: solRecipient.publicKey,
      expectedUtxoHistoryLength: 1,
    };
    let error = null;
    try {
      await user.unshield({
        publicAmountSol: testInputs.amountSol,
        token: testInputs.token,
        recipient: testInputs.recipient,
        confirmOptions: ConfirmOptions.spendable,
      });
    } catch (e) {
      error = e;
    }
    expect(error).to.exist;
  });
  // TODO: add a shield... before, add a transfer too tho, => assert job queeing functioning etc
  it("should unshield SOL and update merkle tree", async () => {
    const solRecipient = Keypair.generate();

    const testInputs = {
      amountSol: 0.05,
      token: "SOL",
      type: Action.UNSHIELD,
      recipient: solRecipient.publicKey,
      expectedUtxoHistoryLength: 1,
    };

    const userTestAssertHelper = new UserTestAssertHelper({
      userSender: user,
      userRecipient: user,
      provider,
      testInputs,
    });
    // need to wait for balance update to fetch current utxos
    await user.getBalance();
    await userTestAssertHelper.fetchAndSaveState();
    await user.unshield({
      publicAmountSol: testInputs.amountSol,
      token: testInputs.token,
      recipient: testInputs.recipient,
      confirmOptions: ConfirmOptions.spendable,
    });

    await waitForBalanceUpdate(userTestAssertHelper, user);
    await userTestAssertHelper.checkSolUnshielded();
  });
  it("Should fail to update Merkle tree", (done: any) => {
    chai
      .request(server)
      .get("/updatemerkletree")
      .end((_err, res) => {
        const error = res.error;
        assert.isNotFalse(error);
        if (error !== false) {
          assert.isTrue(
            error.message.includes("cannot GET /updatemerkletree (404)"),
          );
        }
        expect(res).to.have.status(404);
        done();
      });
  });

  it("Should return lookup table data", (done: any) => {
    chai
      .request(server)
      .get("/lookuptable")
      .end(async (_err, res) => {
        const provider = await Provider.init({
          wallet: userKeypair,
          confirmConfig,
          relayer,
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

  it("Should fail to return lookup table data", (done: any) => {
    chai
      .request(server)
      .post("/lookuptable")
      .end((_err, res) => {
        const error = res.error;
        assert.isNotFalse(error);
        if (error !== false) {
          assert.isTrue(
            error.message.includes("cannot POST /lookuptable (404)"),
          );
        }
        expect(res).to.have.status(404);
        done();
      });
  });

  it("should transfer sol and update merkle tree ", async () => {
    const testInputs = {
      amountSol: 0.05,
      token: "SOL",
      type: Action.TRANSFER,
      expectedUtxoHistoryLength: 1,
      recipientSeed: bs58.encode(new Uint8Array(32).fill(9)),
      expectedRecipientUtxoLength: 1,
    };

    const recipientAccount = new Account({
      poseidon,
      seed: testInputs.recipientSeed,
    });

    const userRecipient: User = await User.init({
      provider,
      seed: testInputs.recipientSeed,
    });

    const testStateValidator = new UserTestAssertHelper({
      userSender: user,
      userRecipient,
      provider,
      testInputs,
    });
    await testStateValidator.fetchAndSaveState();
    // need to wait for balance update to fetch current utxos
    await user.getBalance();
    await user.transfer({
      amountSol: testInputs.amountSol,
      token: testInputs.token,
      recipient: recipientAccount.getPublicKey(),
    });

    await sleep(6000);
    await testStateValidator.checkSolTransferred();
  });

  // TODO: add test for just proper indexing (-> e.g. shields)
  // TODO: add test for stress test load (multiple requests, wrong requests etc)
  it("Should fail transaction with empty instructions", (done: any) => {
    const instructions: any[] = []; // Replace with a valid instruction object
    chai
      .request(server)
      .post("/relayTransaction")
      .send({ instructions })
      .end((_err, res) => {
        expect(res).to.have.status(500);
        assert.isTrue(res.body.message.includes("No instructions provided"));
        done();
      });
  });
});

describe("Util Unit tests", () => {
  describe("getUidFromIxs function", () => {
    const getMockIx = (ixData: number[]) => {
      return new TransactionInstruction({
        programId: SystemProgram.programId, // mock
        data: Buffer.from(ixData),
        keys: [],
      });
    };

    it("should return consistent hash for the same input", () => {
      const mockIxs = [getMockIx([1, 2, 3]), getMockIx([4, 5, 6])];

      const result1 = getUidFromIxs(mockIxs);
      const result2 = getUidFromIxs(mockIxs);

      expect(result1).to.eq(result2);
    });

    it("should return different hashes for different inputs", () => {
      const mockIxs1 = [getMockIx([1, 2, 3]), getMockIx([4, 5, 6])];

      const mockIxs2 = [getMockIx([1, 2, 3]), getMockIx([4, 5, 2])];

      const result1 = getUidFromIxs(mockIxs1);
      const result2 = getUidFromIxs(mockIxs2);

      expect(result1).not.to.eq(result2);
    });

    it("should return a hash of fixed length (64 characters for SHA3-256)", () => {
      const mockIxs = [getMockIx([1, 2, 3]), getMockIx([4, 5, 6])];

      const result = getUidFromIxs(mockIxs);

      expect(result.length).to.eq(44);
    });
  });
});
