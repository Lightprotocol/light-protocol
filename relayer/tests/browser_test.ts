//@ts-check
import { AnchorProvider, BN } from "@coral-xyz/anchor";
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

    const encodedMessage = new TextEncoder().encode(message);
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

    const user = await User.init({
      provider,
      seed: bs58.encode(signature),
    });

    await airdropSol({
      connection: provider.provider.connection,
      recipientPublicKey: walletMock.publicKey!,
      lamports: 4e9,
    });
    await user.shield({
      publicAmountSol: "3",
      token: "SOL",
    });

    const testRecipientKeypair = SolanaKeypair.generate();
    await airdropSol({
      connection: provider.connection!,
      lamports: 2e9,
      recipientPublicKey: testRecipientKeypair.publicKey,
    });
    const lightProviderRecipient = await Provider.init({
      wallet: testRecipientKeypair,
      relayer: RELAYER,
      confirmConfig,
    });

    const testRecipient = await User.init({
      provider: lightProviderRecipient,
    });

    await user.transfer({
      amountSol: "0.25",
      token: "SOL",
      recipient: testRecipient.account.getPublicKey(),
    });

    await user.unshield({
      publicAmountSol: "1.5",
      token: "SOL",
      recipient: new PublicKey("ErAe2LmEKgBNCSP7iA8Z6B396yB6LGCUjzuPrczJYDbz"),
    });
  });
});
