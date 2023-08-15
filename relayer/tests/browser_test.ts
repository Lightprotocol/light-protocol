//@ts-check
import { AnchorProvider, BN, utils } from "@coral-xyz/anchor";
import {
  Connection,
  Keypair as SolanaKeypair,
  PublicKey,
} from "@solana/web3.js";
import chai, { expect } from "chai";
import chaiHttp from "chai-http";
import express from "express";
import { sign } from "tweetnacl";

import {
  Provider,
  airdropSol,
  User,
  Relayer,
  useWallet,
  ADMIN_AUTH_KEYPAIR,
  createTestAccounts,
  confirmConfig,
  UserTestAssertHelper,
  Action,
  sleep,
  Account,
} from "@lightprotocol/zk.js";
import sinon from "sinon";

import {
  updateMerkleTree,
  getIndexedTransactions,
  handleRelayRequest,
  buildMerkleTree,
  getLookUpTable,
} from "../src/services";
import { bs58 } from "@coral-xyz/anchor/dist/cjs/utils/bytes";
let circomlibjs = require("circomlibjs");

chai.use(chaiHttp);
const app = express();
app.use(express.json());
app.use(express.urlencoded({ extended: false }));

// Use sinon to create a stub for the middleware
const addCorsHeadersStub = sinon
  .stub()
  .callsFake((_req: any, _res: any, next: any) => next());
app.use(addCorsHeadersStub);

app.post("/updatemerkletree", updateMerkleTree);
app.get("/lookuptable", getLookUpTable);
app.post("/relayTransaction", handleRelayRequest);
app.get("/indexedTransactions", getIndexedTransactions);
app.get("/getBuiltMerkletree", buildMerkleTree);

describe("Browser tests", () => {
  var RELAYER: Relayer;
  const walletMock = useWallet(ADMIN_AUTH_KEYPAIR);
  const connection = new Connection("http://127.0.0.1:8899", "confirmed");

  before(async () => {
    await createTestAccounts(connection);

    const relayerRecipientSol = SolanaKeypair.generate().publicKey;
    await connection.requestAirdrop(relayerRecipientSol, 2_000_000_000);
    let relayer = SolanaKeypair.generate();
    await airdropSol({
      connection: connection,
      lamports: 2_000_000_000,
      recipientPublicKey: relayer.publicKey,
    });

    // TODO: This will only work as long as .env keys don't change
    RELAYER = new Relayer(
      new PublicKey("EkXDLi1APzu6oxJbg5Hnjb24kfKauJp1xCb5FAUMxf9D"),
      new PublicKey("AV3LnV78ezsEBZebNeMPtEcH1hmvSfUBC5Xbyrzqbt44"),
      new BN(100000),
    );
    await airdropSol({
      connection: connection,
      lamports: 2_000_000_000,
      recipientPublicKey: walletMock.publicKey,
    });
  });
  it("should fail to init node feature (anchorprovider)", async () => {
    // should expect to throw
    expect(() => {
      AnchorProvider.local("http://127.0.0.1:8899", confirmConfig);
    }).to.throw("Provider local is not available on browser.");
  });

  it("should init user, shield, transfer, unshield", async () => {
    const message =
      "IMPORTANT:\nThe application will be able to spend \nyour shielded assets. \n\nOnly sign the message if you trust this\n application.\n\n View all verified integrations here: \n'https://docs.lightprotocol.com/partners'";

    const encodedMessage = utils.bytes.utf8.encode(message);
    const signature = await walletMock.signMessage(encodedMessage);
    if (
      !sign.detached.verify(
        encodedMessage,
        signature,
        walletMock.publicKey.toBytes(),
      )
    )
      throw new Error("Invalid signature!");

    if (!walletMock.signMessage) throw new Error("Wallet not connected!");
    const provider = await Provider.init({
      relayer: RELAYER,
      wallet: walletMock,
      confirmConfig,
    });

    const user: User = await User.init({
      provider,
      seed: bs58.encode(signature),
    });

    await airdropSol({
      connection: provider.provider.connection,
      recipientPublicKey: walletMock.publicKey!,
      lamports: 4e9,
    });

    // because we're running functional_tests before on the same validator
    let utxoHistory = await user.getTransactionHistory();

    let expectedUtxoHistory = utxoHistory.length || 0;

    let testInputs = {
      amountSol: 3,
      token: "SOL",
      type: Action.SHIELD,
      expectedUtxoHistoryLength: expectedUtxoHistory++,
    };

    const testStateValidator = new UserTestAssertHelper({
      userSender: user,
      userRecipient: user,
      provider,
      testInputs,
    });

    await testStateValidator.fetchAndSaveState();

    await user.shield({
      publicAmountSol: testInputs.amountSol,
      token: testInputs.token,
    });

    await testStateValidator.checkSolShielded();

    // TRANSFER
    const testInputsTransfer = {
      amountSol: 1,
      token: "SOL",
      type: Action.TRANSFER,
      expectedUtxoHistoryLength: expectedUtxoHistory++,
      recipientSeed: bs58.encode(new Uint8Array(32).fill(9)),
      expectedRecipientUtxoLength: 1,
    };
    let poseidon = await circomlibjs.buildPoseidonOpt();

    const recipientAccount = new Account({
      poseidon,
      seed: testInputsTransfer.recipientSeed,
    });

    const userRecipient: User = await User.init({
      provider,
      seed: testInputsTransfer.recipientSeed,
    });

    const testStateValidatorTransfer = new UserTestAssertHelper({
      userSender: user,
      userRecipient,
      provider,
      testInputs: testInputsTransfer,
    });
    await testStateValidatorTransfer.fetchAndSaveState();
    // need to wait for balance update to fetch current utxos
    await user.getBalance();
    await user.transfer({
      amountSol: testInputsTransfer.amountSol,
      token: testInputsTransfer.token,
      recipient: recipientAccount.getPublicKey(),
    });

    // await waitForBalanceUpdate(testStateValidator, user);
    await sleep(6000);
    await testStateValidatorTransfer.checkSolTransferred();
    // const testRecipientKeypair = SolanaKeypair.generate();
    // await airdropSol({
    //   connection: provider.connection!,
    //   lamports: 2e9,
    //   recipientPublicKey: testRecipientKeypair.publicKey,
    // });
    // const lightProviderRecipient = await Provider.init({
    //   wallet: testRecipientKeypair,
    //   relayer: RELAYER,
    //   confirmConfig,
    // });

    // const testRecipient = await User.init({
    //   provider: lightProviderRecipient,
    // });

    // let testInputsTransfer = {
    //   amountSol: 0.25,
    //   token: "SOL",
    //   type: Action.TRANSFER,
    //   expectedUtxoHistoryLength: expectedUtxoHistory++,
    // };

    // const testStateValidatorTransfer = new UserTestAssertHelper({
    //   userSender: user,
    //   userRecipient: testRecipient.account.getPublicKey(),
    //   provider,
    //   testInputs: testInputsTransfer,
    // });

    // await testStateValidatorTransfer.fetchAndSaveState();

    // await user.transfer({
    //   recipient: testRecipient.account.getPublicKey(),
    //   amountSol: testInputsTransfer.amountSol,
    //   token: testInputsTransfer.token,
    // });

    // await testStateValidatorTransfer.checkSolTransferred();

    let testInputsUnshield = {
      amountSol: 1.5,
      token: "SOL",
      type: Action.UNSHIELD,
      expectedUtxoHistoryLength: expectedUtxoHistory++,
    };

    const testStateValidatorUnshield = new UserTestAssertHelper({
      userSender: user,
      userRecipient: user,
      provider,
      testInputs: testInputsUnshield,
    });

    await testStateValidatorUnshield.fetchAndSaveState();

    await user.unshield({
      publicAmountSol: testInputsUnshield.amountSol,
      token: testInputsUnshield.token,
      recipient: provider.wallet.publicKey,
    });
  });
});
