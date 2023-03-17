import * as anchor from "@coral-xyz/anchor";
import { Connection, PublicKey } from "@solana/web3.js";
import _ from "lodash";

// TODO: add and use  namespaces in SDK
import {
  ADMIN_AUTH_KEYPAIR,
  Provider,
  User,
  strToArr,
  updateMerkleTreeForTest,
} from "light-sdk";
import { sign } from "tweetnacl";
import { executeWithInput, runCommand } from "./mock/cmd";

const sleep = (ms: number) => {
  return new Promise((resolve) => setTimeout(resolve, ms));
};

// TODO: remove deprecated function calls
describe("browser_wallet", () => {
  let connection;

  before(async () => {
    connection = new Connection("http://127.0.0.1:8899");
    const response = executeWithInput(
      "cd ../relayer && node lib/index.js && ../",
    );
    await sleep(20000)
  });

  const userKeypair = ADMIN_AUTH_KEYPAIR; //new SolanaKeypair();

  const signTransaction = async (tx) => {
    await tx.sign([userKeypair!]);
    return tx;
  };

  const signMessage = async (message) => {
    return sign.detached(message, userKeypair.secretKey);
  };

  const sendAndConfirmTransaction = async (fn) => {
    return await fn();
  };

  it("(user class) shield SOL", async () => {

    let amount = 15;
    let token = "SOL";

    const provider = await Provider.browser(
      {
        signMessage,
        signTransaction,
        sendAndConfirmTransaction,
        publicKey: userKeypair.publicKey,
      },
      connection,
    ); // userKeypair

    let res = await provider.provider.connection.requestAirdrop(
      userKeypair.publicKey,
      4_000_000_000,
    );

    await provider.provider.connection.confirmTransaction(res, "confirmed");

    const user = await User.load(provider);

    await user.shield({ amount, token });
  });

  it("(user class) unshield SOL", async () => {
    let amount = 1;
    let token = "SOL";
    let recipient = new PublicKey(
      "E7jqevikamCMCda8yCsfNawj57FSotUZuref9MLZpWo1",
    );
    const provider = await Provider.browser(
      {
        signMessage,
        signTransaction,
        sendAndConfirmTransaction,
        publicKey: userKeypair.publicKey,
      },
      connection,
    ); // userKeypair
    const user = await User.load(provider);
    await user.unshield({ amount, token, recipient });
  });

  it("(user class) transfer SOL", async () => {
    let amount = 1;
    let token = "SOL";
    const shieldedRecipient =
      "19a20668193c0143dd96983ef457404280741339b95695caddd0ad7919f2d434";
    const encryptionPublicKey =
      "LPx24bc92eecaf5e3904bc1f4f731a2b1e0a28adf445e800c4cff112eb7a3f5350b";

    const recipient = new anchor.BN(shieldedRecipient, "hex");
    const recipientEncryptionPublicKey: Uint8Array =
      strToArr(encryptionPublicKey);

    const provider = await Provider.browser(
      {
        signMessage,
        signTransaction,
        sendAndConfirmTransaction,
        publicKey: userKeypair.publicKey,
      },
      connection,
    ); // userKeypair

    console.log("provider ======>");
    const user = await User.load(provider);
    await user.transfer({
      amount,
      token,
      recipient,
      recipientEncryptionPublicKey,
    });
  });
});
