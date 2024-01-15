//@ts-check
import { AnchorProvider, utils } from "@coral-xyz/anchor";
import { Connection, Keypair as SolanaKeypair, Keypair } from "@solana/web3.js";
import chai, { expect } from "chai";
import chaiHttp from "chai-http";
import { sign } from "tweetnacl";

import {
  Provider,
  airdropSol,
  User,
  Rpc,
  useWallet,
  ADMIN_AUTH_KEYPAIR,
  createTestAccounts,
  confirmConfig,
  UserTestAssertHelper,
  Action,
  Account,
  ConfirmOptions,
} from "@lightprotocol/zk.js";
import { bs58 } from "@coral-xyz/anchor/dist/cjs/utils/bytes";
import { waitForBalanceUpdate } from "./test-utils/waitForBalanceUpdate";
import { SOLANA_RPC_URL } from "../src/config";
import { getRpc } from "../src/utils/provider";
import { WasmFactory, LightWasm } from "@lightprotocol/account.rs";

chai.use(chaiHttp);

describe("Browser tests", () => {
  let RPC: Rpc;
  let lightWasm: LightWasm;
  let provider: Provider;
  let user: User;
  const walletMock = useWallet(ADMIN_AUTH_KEYPAIR, SOLANA_RPC_URL);
  const connection = new Connection(SOLANA_RPC_URL, "confirmed");

  before(async () => {
    lightWasm = await WasmFactory.getInstance();

    await createTestAccounts(connection);

    const rpcRecipientSol = SolanaKeypair.generate().publicKey;
    await connection.requestAirdrop(rpcRecipientSol, 9e8);
    const rpc = SolanaKeypair.generate();
    await airdropSol({
      connection: connection,
      lamports: 9e8,
      recipientPublicKey: rpc.publicKey,
    });

    RPC = getRpc();
    await airdropSol({
      connection: connection,
      lamports: 9e8,
      recipientPublicKey: walletMock.publicKey,
    });

    const message =
      "IMPORTANT:\nThe application will be able to spend \nyour compressed assets. \n\nOnly sign the message if you trust this\n application.\n\n View all verified integrations here: \n'https://docs.lightprotocol.com/partners'";

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
    provider = await Provider.init({
      rpc: RPC,
      wallet: walletMock,
      confirmConfig,
    });

    user = await User.init({
      provider,
      seed: bs58.encode(signature),
    });

    await airdropSol({
      connection: provider.provider.connection,
      recipientPublicKey: walletMock.publicKey!,
      lamports: 9e8,
    });
  });

  it("should fail to init node feature (anchorprovider)", async () => {
    // should expect to throw
    expect(() => {
      AnchorProvider.local("http://127.0.0.1:8899", confirmConfig);
    }).to.throw("Provider local is not available on browser.");
  });

  it("(browser) should compress and update merkle tree", async () => {
    const testInputs = {
      amountSol: 0.2,
      token: "SOL",
      type: Action.COMPRESS,
      expectedUtxoHistoryLength: 1,
    };
    const userTestAssertHelper = new UserTestAssertHelper({
      userSender: user,
      userRecipient: user,
      provider,
      testInputs,
    });
    await userTestAssertHelper.fetchAndSaveState();

    await user.compress({
      publicAmountSol: testInputs.amountSol,
      token: testInputs.token,
      confirmOptions: ConfirmOptions.spendable,
    });

    await waitForBalanceUpdate(userTestAssertHelper, user);
    await userTestAssertHelper.checkSolShielded();
  });

  it("(browser) should decompress SOL and update merkle tree", async () => {
    const solRecipient = Keypair.generate();

    const testInputs = {
      amountSol: 0.05,
      token: "SOL",
      type: Action.DECOMPRESS,
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
    await user.decompress({
      publicAmountSol: testInputs.amountSol,
      token: testInputs.token,
      recipient: testInputs.recipient,
      confirmOptions: ConfirmOptions.spendable,
    });

    await waitForBalanceUpdate(userTestAssertHelper, user);
    await userTestAssertHelper.checkSolUnshielded();
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

    await waitForBalanceUpdate(testStateValidator, user);
    await testStateValidator.checkSolTransferred();
  });
});
