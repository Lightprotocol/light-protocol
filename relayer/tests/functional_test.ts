//@ts-check
import { AnchorProvider, BN } from "@coral-xyz/anchor";
import {
  Keypair,
  PublicKey,
  SystemProgram,
  TransactionInstruction,
} from "@solana/web3.js";
import chai, { assert } from "chai";
import chaiHttp from "chai-http";

import {
  Account,
  Provider,
  airdropSol,
  confirmConfig,
  User,
  sleep,
  Action,
  UserTestAssertHelper,
  ConfirmOptions,
  Relayer,
  RELAYER_FEE,
  TOKEN_ACCOUNT_FEE,
  MerkleTreeConfig,
  merkleTreeProgramId,
  AUTHORITY,
  getVerifierProgramId,
  IDL_LIGHT_PSP2IN2OUT,
  MINT,
} from "@lightprotocol/zk.js";

import { getUidFromIxs } from "../src/services";
import { getKeyPairFromEnv } from "../src/utils/provider";
import { waitForBalanceUpdate } from "./test-utils/waitForBalanceUpdate";
import { RELAYER_URL } from "../src/config";
import { WasmFactory, LightWasm } from "@lightprotocol/account.rs";
const bs58 = require("bs58");

chai.use(chaiHttp);
const expect = chai.expect;
const server = RELAYER_URL;

describe("API tests", () => {
  let lightWasm: LightWasm;
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
    lightWasm = await WasmFactory.getInstance();
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

  it("should shield", async () => {
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

  it("getEventById", (done) => {
    const utxo = user.getAllUtxos()[0];
    const requestData = {
      id: bs58.encode(
        user.account.generateUtxoPrefixHash(
          MerkleTreeConfig.getTransactionMerkleTreePda(),
          0,
        ),
      ),
      merkleTreePdaPublicKey:
        MerkleTreeConfig.getTransactionMerkleTreePda().toBase58(),
    };
    chai
      .request(server)
      .post("/getEventById")
      .send(requestData)
      .end((_err, res) => {
        expect(res).to.have.status(200);
        expect(res.body.data).to.exist;
        expect(res.body.data).to.have.property("transaction");
        assert.deepEqual(res.body.data.leavesIndexes, [0, 1]);
        assert.equal(res.body.data.merkleProofs.length, 2);
        assert.equal(res.body.data.merkleProofs[0].length, 18);
        assert.equal(res.body.data.merkleProofs[1].length, 18);
        assert.equal(
          res.body.data.transaction.signer,
          user.provider.wallet.publicKey.toBase58(),
        );
        assert.equal(
          res.body.data.transaction.to,
          MerkleTreeConfig.getSolPoolPda(merkleTreeProgramId).pda.toBase58(),
        );

        console.log("to spl", res.body.data.transaction.toSpl);
        console.log(
          "ref ",
          MerkleTreeConfig.getSplPoolPdaToken(
            MINT,
            merkleTreeProgramId,
          ).toBase58(),
        );
        console.log(
          "ref pool type ",
          MerkleTreeConfig.getSolPoolPda(MINT).pda.toBase58(),
        );
        // TODO: investigate what account is used as place holder here
        // assert.equal(res.body.data.transaction.toSpl, MerkleTreeConfig.getSplPoolPdaToken(MINT, merkleTreeProgramId).toBase58());
        assert.equal(res.body.data.transaction.fromSpl, AUTHORITY.toBase58());
        assert.equal(
          res.body.data.transaction.verifier,
          getVerifierProgramId(IDL_LIGHT_PSP2IN2OUT).toBase58(),
        );
        assert.equal(res.body.data.transaction.type, Action.SHIELD);

        assert.equal(
          res.body.data.transaction.publicAmountSol,
          new BN(3e8).toString("hex"),
        );
        assert.equal(res.body.data.transaction.publicAmountSpl, "0");
        assert.equal(
          new BN(res.body.data.transaction.leaves[0]).toString(),
          utxo.getCommitment(lightWasm),
        );
        assert.equal(res.body.data.transaction.firstLeafIndex, "0");
        // don't assert nullifiers since we shielded
        // assert.deepEqual(res.body.data.transaction.nullifiers[0], new BN(utxo.getNullifier({hasher,account: user.account}).toString()).toArray("be", 32));
        assert.equal(res.body.data.transaction.relayerFee, "0");

        console.log("change amount", res.body.data.transaction.changeSolAmount);
        done();
      });
  });

  it("getEventsByIdBatch", (done) => {
    const utxo = user.getAllUtxos()[0];
    const requestData = {
      ids: [
        bs58.encode(
          user.account.generateUtxoPrefixHash(
            MerkleTreeConfig.getTransactionMerkleTreePda(),
            0,
          ),
        ),
      ],
      merkleTreePdaPublicKey:
        MerkleTreeConfig.getTransactionMerkleTreePda().toBase58(),
    };
    chai
      .request(server)
      .post("/getEventsByIdBatch")
      .send(requestData)
      .end((_err, res) => {
        expect(res).to.have.status(200);
        expect(res.body.data).to.exist;
        expect(res.body.data[0]).to.have.property("transaction");
        assert.deepEqual(res.body.data[0].leavesIndexes, [0, 1]);
        assert.equal(res.body.data[0].merkleProofs.length, 2);
        assert.equal(res.body.data[0].merkleProofs[0].length, 18);
        assert.equal(res.body.data[0].merkleProofs[1].length, 18);
        assert.equal(
          res.body.data[0].transaction.signer,
          user.provider.wallet.publicKey.toBase58(),
        );
        assert.equal(
          res.body.data[0].transaction.to,
          MerkleTreeConfig.getSolPoolPda(merkleTreeProgramId).pda.toBase58(),
        );
        assert.equal(
          res.body.data[0].transaction.fromSpl,
          AUTHORITY.toBase58(),
        );
        assert.equal(
          res.body.data[0].transaction.verifier,
          getVerifierProgramId(IDL_LIGHT_PSP2IN2OUT).toBase58(),
        );
        assert.equal(res.body.data[0].transaction.type, Action.SHIELD);

        assert.equal(
          res.body.data[0].transaction.publicAmountSol,
          new BN(3e8).toString("hex"),
        );
        assert.equal(res.body.data[0].transaction.publicAmountSpl, "0");
        assert.equal(
          new BN(res.body.data[0].transaction.leaves[0]).toString(),
          utxo.getCommitment(lightWasm),
        );
        assert.equal(res.body.data[0].transaction.firstLeafIndex, "0");
        // don't assert nullifiers since we shielded
        // assert.equal(new BN(res.body.data[0].transaction.nullifiers[0], 32, "be").toString(), utxo.getNullifier({hasher,account: user.account, index: 0}));
        assert.equal(res.body.data[0].transaction.relayerFee, "0");

        console.log(
          "change amount",
          res.body.data[0].transaction.changeSolAmount,
        );
        done();
      });
  });

  it("getMerkleRoot", (done) => {
    const requestData = {
      merkleTreePdaPublicKey:
        MerkleTreeConfig.getTransactionMerkleTreePda().toBase58(),
    };
    chai
      .request(server)
      .post("/getMerkleRoot")
      .send(requestData)
      .end((_err, res) => {
        expect(res).to.have.status(200);
        expect(res.body.data).to.exist;
        expect(res.body.data).to.have.property("root");
        expect(res.body.data).to.have.property("index");
        assert.equal(res.body.data.index, 1);
        done();
      });
  });

  it("getMerkleProofByIndexBatch", (done) => {
    const requestData = {
      merkleTreePdaPublicKey:
        MerkleTreeConfig.getTransactionMerkleTreePda().toBase58(),
      indexes: [0, 1],
    };
    chai
      .request(server)
      .post("/getMerkleProofByIndexBatch")
      .send(requestData)
      .end((_err, res) => {
        console.log("res.body.data", res.body);
        expect(res).to.have.status(200);
        expect(res.body.data).to.exist;
        expect(res.body.data).to.have.property("root");
        expect(res.body.data).to.have.property("index");
        assert.equal(res.body.data.index, 1);
        assert.equal(res.body.data.merkleProofs.length, 2);
        assert.equal(res.body.data.merkleProofs[0].length, 18);
        assert.equal(res.body.data.merkleProofs[1].length, 18);
        done();
      });
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
  it("should unshield SOL", async () => {
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
    try {
      await userTestAssertHelper.checkSolUnshielded();
    } catch (e) {
      console.log("userTestAssertHelper ", userTestAssertHelper);
      console.log("\n----------------------------------\n");
      console.log(user.getBalance());
      throw e;
    }
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

  it("should transfer sol", async () => {
    const testInputs = {
      amountSol: 0.05,
      token: "SOL",
      type: Action.TRANSFER,
      expectedUtxoHistoryLength: 1,
      recipientSeed: bs58.encode(new Uint8Array(32).fill(9)),
      expectedRecipientUtxoLength: 1,
    };

    const recipientAccount = Account.createFromSeed(
      lightWasm,
      testInputs.recipientSeed,
    );

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

  // Test to debug flakyness in unshield test
  it.skip("debug should unshield SOL", async () => {
    for (let i = 0; i < 20; i++) {
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
      const solRecipient = Keypair.generate();

      const testInputsUnshield = {
        amountSol: 0.05,
        token: "SOL",
        type: Action.UNSHIELD,
        recipient: solRecipient.publicKey,
        expectedUtxoHistoryLength: 1,
      };
      await user.getBalance();
      const userTestAssertHelperUnshield = new UserTestAssertHelper({
        userSender: user,
        userRecipient: user,
        provider,
        testInputs: testInputsUnshield,
      });
      // need to wait for balance update to fetch current utxos
      await userTestAssertHelperUnshield.fetchAndSaveState();
      await user.unshield({
        publicAmountSol: testInputsUnshield.amountSol,
        token: testInputsUnshield.token,
        recipient: testInputsUnshield.recipient,
        confirmOptions: ConfirmOptions.spendable,
      });

      await waitForBalanceUpdate(userTestAssertHelperUnshield, user);
      try {
        await userTestAssertHelperUnshield.checkSolUnshielded();
      } catch (e) {
        console.log(
          "userTestAssertHelperUnshield ",
          userTestAssertHelperUnshield,
        );
        console.log("\n----------------------------------\n");
        console.log(await user.getBalance());
        throw e;
      }
    }
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
