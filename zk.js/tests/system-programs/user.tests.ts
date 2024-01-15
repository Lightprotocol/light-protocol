import { Keypair, Keypair as SolanaKeypair } from "@solana/web3.js";
import { sign } from "tweetnacl";
const chai = require("chai");
const chaiAsPromised = require("chai-as-promised");
// init chai-as-promised support
chai.use(chaiAsPromised);

import { bs58 } from "@coral-xyz/anchor/dist/cjs/utils/bytes";

import {
  ADMIN_AUTH_KEYPAIR,
  Provider,
  createTestAccounts,
  confirmConfig,
  User,
  Account,
  CreateUtxoErrorCode,
  UserErrorCode,
  TransactionErrorCode,
  ADMIN_AUTH_KEY,
  TestRpc,
  Action,
  UserTestAssertHelper,
  generateRandomTestAmount,
  airdropSol,
  ConfirmOptions,
  airdropCompressedSol,
  TOKEN_ACCOUNT_FEE,
  useWallet,
  RPC_FEE,
  BN_1,
} from "../../src";
import { WasmFactory, LightWasm } from "@lightprotocol/account.rs";
import { AnchorProvider, setProvider } from "@coral-xyz/anchor";
import { expect } from "chai";

let WASM: LightWasm, RPC: TestRpc, provider: Provider, user: User;

describe("Test User", () => {
  // Configure the client to use the local cluster.
  process.env.ANCHOR_WALLET = process.env.HOME + "/.config/solana/id.json";
  process.env.ANCHOR_PROVIDER_URL = "http://127.0.0.1:8899";

  const anchorProvider = AnchorProvider.local(
    "http://127.0.0.1:8899",
    confirmConfig,
  );
  setProvider(anchorProvider);
  const userKeypair = ADMIN_AUTH_KEYPAIR;

  before("init test setup Merkle tree lookup table etc ", async () => {
    await createTestAccounts(anchorProvider.connection);
    WASM = await WasmFactory.getInstance();

    const rpcRecipientSol = SolanaKeypair.generate().publicKey;
    await anchorProvider.connection.requestAirdrop(
      rpcRecipientSol,
      2_000_000_000,
    );
    const rpc = Keypair.generate();
    await airdropSol({
      connection: anchorProvider.connection,
      lamports: 2_000_000_000,
      recipientPublicKey: rpc.publicKey,
    });
    RPC = new TestRpc({
      rpcPubkey: rpc.publicKey,
      rpcRecipientSol,
      rpcFee: RPC_FEE,
      highRpcFee: TOKEN_ACCOUNT_FEE,
      payer: rpc,
      connection: anchorProvider.connection,
      lightWasm: WASM,
    });

    provider = await Provider.init({
      wallet: userKeypair,
      rpc: RPC,
      confirmConfig,
    });
    await airdropSol({
      connection: anchorProvider.connection,
      lamports: 2_000_000_000,
      recipientPublicKey: userKeypair.publicKey,
    });

    user = await User.init({ provider });
  });

  it("externally supplied seed vs internal seed (user derivation)", async () => {
    const message =
      "IMPORTANT:\nThe application will be able to spend \nyour compressed assets. \n\nOnly sign the message if you trust this\n application.\n\n View all verified integrations here: \n'https://docs.lightprotocol.com/partners'";

    const walletMock = useWallet(ADMIN_AUTH_KEYPAIR);
    const walletMock2 = useWallet(Keypair.generate());

    const encodedMessage = new TextEncoder().encode(message);
    const signature = await walletMock.signMessage(encodedMessage);
    const signature2 = await walletMock2.signMessage(encodedMessage);

    if (
      !sign.detached.verify(
        encodedMessage,
        signature,
        walletMock.publicKey.toBytes(),
      )
    )
      throw new Error("Invalid signature!");

    const testRpc = new TestRpc({
      rpcPubkey: ADMIN_AUTH_KEYPAIR.publicKey,
      rpcRecipientSol: Keypair.generate().publicKey,
      rpcFee: RPC_FEE,
      payer: ADMIN_AUTH_KEYPAIR,
      connection: anchorProvider.connection,
      lightWasm: WASM,
    });
    const providerExternalSeed = await Provider.init({
      rpc: testRpc,
      wallet: walletMock,
      confirmConfig,
    });
    const providerInternalSeed = await Provider.init({
      rpc: testRpc,
      wallet: walletMock,
      confirmConfig,
    });

    const userExternal = await User.init({
      provider: providerExternalSeed,
      seed: bs58.encode(signature),
    });
    const userExternal2 = await User.init({
      provider: providerExternalSeed,
      seed: bs58.encode(signature2),
    });
    const userInternal = await User.init({
      provider: providerInternalSeed,
    });

    const externalKey = userExternal.account.getPublicKey();
    const externalKey2 = userExternal2.account.getPublicKey();
    const internalKey = userInternal.account.getPublicKey();

    expect(externalKey).to.deep.equal(internalKey);
    expect(externalKey2).to.not.deep.equal(internalKey);
  });

  it("(user class) compress SPL", async () => {
    const expectedSpentUtxosLength = 0;
    const expectedUtxoHistoryLength = 1;
    const testInputs = {
      amountSpl: generateRandomTestAmount(0, 100000, 2),
      token: "USDC",
      type: Action.COMPRESS,
      expectedUtxoHistoryLength,
      expectedSpentUtxosLength,
    };

    const testStateValidator = new UserTestAssertHelper({
      userSender: user,
      userRecipient: user,
      provider,
      testInputs,
    });

    await testStateValidator.fetchAndSaveState();

    await user.compress({
      publicAmountSpl: testInputs.amountSpl,
      token: testInputs.token,
    });
    await testStateValidator.checkSplCompressed();
  });

  it("(user class) compress SOL", async () => {
    const testInputs = {
      amountSol: 15,
      token: "SOL",
      type: Action.COMPRESS,
      expectedUtxoHistoryLength: 1,
    };

    const testStateValidator = new UserTestAssertHelper({
      userSender: user,
      userRecipient: user,
      provider,
      testInputs,
    });

    await testStateValidator.fetchAndSaveState();

    await user.compress({
      publicAmountSol: testInputs.amountSol,
      token: testInputs.token,
    });

    await testStateValidator.checkSolCompressed();
  });

  it("(user class) decompress SPL", async () => {
    const solRecipient = SolanaKeypair.generate();

    const testInputs = {
      amountSpl: 1,
      token: "USDC",
      type: Action.DECOMPRESS,
      recipient: solRecipient.publicKey,
      expectedUtxoHistoryLength: 1,
    };

    const testStateValidator = new UserTestAssertHelper({
      userSender: user,
      userRecipient: user,
      provider,
      testInputs,
    });

    await testStateValidator.fetchAndSaveState();

    await user.decompress({
      publicAmountSpl: testInputs.amountSpl,
      token: testInputs.token,
      recipient: testInputs.recipient,
    });

    await testStateValidator.checkSplDecompressed();
  });

  it("(user class) transfer SPL", async () => {
    const testInputs = {
      amountSpl: 1,
      amountSol: 0,
      token: "USDC",
      type: Action.TRANSFER,
      expectedUtxoHistoryLength: 1,
      recipientSeed: bs58.encode(new Uint8Array(32).fill(9)),
      expectedRecipientUtxoLength: 1,
    };

    const recipientAccount = Account.createFromSeed(
      WASM,
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

    await user.compress({
      publicAmountSpl: testInputs.amountSpl,
      token: testInputs.token,
    });

    await testStateValidator.fetchAndSaveState();

    await user.transfer({
      amountSpl: testInputs.amountSpl,
      token: testInputs.token,
      recipient: recipientAccount.getPublicKey(),
    });

    await user.getBalance();
    await testStateValidator.checkSplTransferred();
  });

  it("(user class) storage compress", async () => {
    const testInputs = {
      amountSpl: 0,
      amountSol: 0,
      token: "SOL",
      type: Action.COMPRESS,
      expectedUtxoHistoryLength: 1,
      storage: true,
      message: Buffer.alloc(512).fill(1),
    };

    const testStateValidator = new UserTestAssertHelper({
      userSender: user,
      userRecipient: user,
      provider,
      testInputs,
    });

    await testStateValidator.fetchAndSaveState();
    await user.storeData(testInputs.message, ConfirmOptions.spendable, true);

    await testStateValidator.assertStoredWithCompress();
  });

  it("(user class) storage transfer", async () => {
    const testInputs = {
      amountSpl: 0,
      amountSol: 0,
      token: "SOL",
      type: Action.TRANSFER,
      expectedUtxoHistoryLength: 1,
      storage: true,
      isSender: true,
      message: Buffer.alloc(672).fill(2),
    };

    const seed = bs58.encode(new Uint8Array(32).fill(4));
    await airdropCompressedSol({ provider, amount: 1, seed });

    const user: User = await User.init({ provider, seed });

    const testStateValidator = new UserTestAssertHelper({
      userSender: user,
      userRecipient: user,
      provider,
      testInputs,
    });

    await testStateValidator.fetchAndSaveState();

    await user.storeData(testInputs.message, ConfirmOptions.spendable, false);

    await testStateValidator.assertStoredWithTransfer();
  });
});

describe("Test User Errors", () => {
  // Configure the client to use the local cluster.
  process.env.ANCHOR_WALLET = process.env.HOME + "/.config/solana/id.json";
  process.env.ANCHOR_PROVIDER_URL = "http://127.0.0.1:8899";

  const providerAnchor = AnchorProvider.local(
    "http://127.0.0.1:8899",
    confirmConfig,
  );
  setProvider(providerAnchor);

  const userKeypair = ADMIN_AUTH_KEYPAIR;
  let amount: number,
    token: string,
    rpc: TestRpc,
    provider: Provider,
    user: User;

  before("init test setup Merkle tree lookup table etc ", async () => {
    if ((await providerAnchor.connection.getBalance(ADMIN_AUTH_KEY)) === 0) {
      await createTestAccounts(providerAnchor.connection);
    }

    WASM = await WasmFactory.getInstance();
    amount = 20;
    token = "USDC";

    const anchorProvider = AnchorProvider.local(
      "http://127.0.0.1:8899",
      confirmConfig,
    );
    setProvider(anchorProvider);

    const rpcRecipientSol = SolanaKeypair.generate().publicKey;
    await anchorProvider.connection.requestAirdrop(
      rpcRecipientSol,
      2_000_000_000,
    );

    const rpc = Keypair.generate();
    await airdropSol({
      connection: anchorProvider.connection,
      lamports: 2_000_000_000,
      recipientPublicKey: rpc.publicKey,
    });

    RPC = new TestRpc({
      rpcPubkey: rpc.publicKey,
      rpcRecipientSol,
      rpcFee: RPC_FEE,
      highRpcFee: TOKEN_ACCOUNT_FEE,
      payer: rpc,
      connection: anchorProvider.connection,
      lightWasm: WASM,
    });
    provider = await Provider.init({
      wallet: userKeypair,
      rpc: RPC,
      confirmConfig,
    });

    user = await User.init({ provider });
  });

  it("NO_PUBLIC_AMOUNTS_PROVIDED compress", async () => {
    await chai.assert.isRejected(
      user.compress({ token }),
      CreateUtxoErrorCode.NO_PUBLIC_AMOUNTS_PROVIDED,
    );
  });

  it("TOKEN_UNDEFINED compress", async () => {
    await chai.assert.isRejected(
      // @ts-ignore
      user.compress({ publicAmountSpl: amount }),
      UserErrorCode.TOKEN_UNDEFINED,
    );
  });

  it("INVALID_TOKEN compress", async () => {
    await chai.assert.isRejected(
      // @ts-ignore
      user.compress({ publicAmountSpl: amount, token: "SOL" }),
      UserErrorCode.INVALID_TOKEN,
    );
  });

  it("TOKEN_ACCOUNT_DEFINED compress", async () => {
    await chai.assert.isRejected(
      // @ts-ignore
      user.compress({
        publicAmountSol: amount,
        token: "SOL",
        senderTokenAccount: SolanaKeypair.generate().publicKey,
      }),
      UserErrorCode.TOKEN_ACCOUNT_DEFINED,
    );
  });

  it("TOKEN_NOT_FOUND compress", async () => {
    await chai.assert.isRejected(
      // @ts-ignore
      user.compress({ publicAmountSol: amount, token: "SPL" }),
      UserErrorCode.TOKEN_NOT_FOUND,
    );
  });

  it("TOKEN_NOT_FOUND decompress", async () => {
    await chai.assert.isRejected(
      // @ts-ignore
      user.decompress({ amountSol: amount, token: "SPL" }),
      UserErrorCode.TOKEN_NOT_FOUND,
    );
  });

  it("TOKEN_NOT_FOUND transfer", async () => {
    await chai.assert.isRejected(
      // @ts-ignore
      user.decompress({ amountSol: amount, token: "SPL" }),
      UserErrorCode.TOKEN_NOT_FOUND,
    );
  });

  it("NO_PUBLIC_AMOUNTS_PROVIDED decompress", async () => {
    await chai.assert.isRejected(
      user.decompress({ token }),
      CreateUtxoErrorCode.NO_PUBLIC_AMOUNTS_PROVIDED,
    );
  });

  it("TOKEN_NOT_FOUND decompress", async () => {
    await chai.assert.isRejected(
      // @ts-ignore
      user.decompress({}),
      UserErrorCode.TOKEN_NOT_FOUND,
    );
  });

  it("SOL_RECIPIENT_UNDEFINED decompress", async () => {
    await chai.assert.isRejected(
      // @ts-ignore
      user.decompress({ token: "SOL", publicAmountSol: BN_1 }),
      TransactionErrorCode.SOL_RECIPIENT_UNDEFINED,
    );

    await chai.assert.isRejected(
      // @ts-ignore
      user.decompress({
        token,
        publicAmountSol: BN_1,
        publicAmountSpl: BN_1,
      }),
      TransactionErrorCode.SOL_RECIPIENT_UNDEFINED,
    );
  });

  it("SPL_RECIPIENT_UNDEFINED decompress", async () => {
    await chai.assert.isRejected(
      // @ts-ignore
      user.decompress({ token, publicAmountSpl: BN_1 }),
      TransactionErrorCode.SPL_RECIPIENT_UNDEFINED,
    );
  });

  it("TOKEN_NOT_FOUND compress", async () => {
    await chai.assert.isRejected(
      // @ts-ignore
      user.compress({ publicAmountSol: SolanaKeypair.generate().publicKey }),
      UserErrorCode.TOKEN_NOT_FOUND,
    );
  });

  it("TOKEN_NOT_FOUND transfer", async () => {
    await chai.assert.isRejected(
      // @ts-ignore
      user.transfer({
        recipient: Account.random(WASM).getPublicKey(),
        amountSol: BN_1,
        token: "SPL",
      }),
      UserErrorCode.TOKEN_NOT_FOUND,
    );
  });

  it("COMPRESSED_RECIPIENT_UNDEFINED transfer", async () => {
    await chai.assert.isRejected(
      // @ts-ignore
      user.transfer({}),
      UserErrorCode.COMPRESSED_RECIPIENT_UNDEFINED,
    );
  });

  it("NO_AMOUNTS_PROVIDED transfer", async () => {
    await chai.assert.isRejected(
      // @ts-ignore
      user.transfer({
        recipient: Account.random(WASM).getPublicKey(),
      }),
      UserErrorCode.NO_AMOUNTS_PROVIDED,
    );
  });
});
