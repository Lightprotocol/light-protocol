import { expect } from "chai";
import { sign } from "tweetnacl";
const chai = require("chai");
const chaiAsPromised = require("chai-as-promised");

// Load chai-as-promised support
chai.use(chaiAsPromised);
import { BN } from "@coral-xyz/anchor";
import { it } from "mocha";

import {
  ADMIN_AUTH_KEYPAIR,
  confirmConfig,
  Provider,
  TestRelayer,
  User,
  useWallet,
} from "../src";
import { bs58 } from "@coral-xyz/anchor/dist/cjs/utils/bytes";
import { Keypair } from "@solana/web3.js";
process.env.ANCHOR_PROVIDER_URL = "http://127.0.0.1:8899";
process.env.ANCHOR_WALLET = process.env.HOME + "/.config/solana/id.json";

describe("Test User Initialization", () => {
  it.only("externally supplied seed vs internal seed (user derivation)", async () => {
    const message =
      "IMPORTANT:\nThe application will be able to spend \nyour shielded assets. \n\nOnly sign the message if you trust this\n application.\n\n View all verified integrations here: \n'https://docs.lightprotocol.com/partners'";

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

    let testRelayer = new TestRelayer({
      relayerPubkey: ADMIN_AUTH_KEYPAIR.publicKey,
      relayerRecipientSol: Keypair.generate().publicKey,
      relayerFee: new BN(100_000),
      payer: ADMIN_AUTH_KEYPAIR,
    });
    const providerExternalSeed = await Provider.init({
      relayer: testRelayer,
      wallet: walletMock,
      confirmConfig,
    });
    const providerInternalSeed = await Provider.init({
      relayer: testRelayer,
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

    const externalKey = await userExternal.account.getPublicKey();
    const externalKey2 = await userExternal2.account.getPublicKey();
    const internalKey = await userInternal.account.getPublicKey();

    expect(externalKey).to.deep.equal(internalKey);
    expect(externalKey2).to.not.deep.equal(internalKey);
  });
});
