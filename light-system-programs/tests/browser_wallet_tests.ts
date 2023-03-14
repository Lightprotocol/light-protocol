import * as anchor from "@coral-xyz/anchor";
import {
  Connection,
  Keypair as SolanaKeypair,
  PublicKey,
} from "@solana/web3.js";
import _ from "lodash";

// TODO: add and use  namespaces in SDK
import {
  ADMIN_AUTH_KEYPAIR,
  MINT,
  Provider,
  newAccountWithTokens,
  User,
  strToArr,
  updateMerkleTreeForTest,
} from "light-sdk";
import { useWallet } from "./mock/useWalletMock";

// TODO: remove deprecated function calls
describe("browser_wallet", () => {
  let connection;

  before(() => {
    connection = new Connection("http://127.0.0.1:8899");
  });

  const userKeypair = ADMIN_AUTH_KEYPAIR; //new SolanaKeypair();
  const { signTransaction, signMessage, sendAndConfirmTransaction } = useWallet(
    userKeypair,
    connection,
  );


  it("(user class) shield SPL", async () => {
    console.log("inside this test")
    let amount = 20;

    let token = "USDC";

    console.log("test user wallet: ", userKeypair.publicKey.toBase58());

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
      2_000_000_000,
    );

    await provider.provider.connection.confirmTransaction(res, "confirmed");

    const user = await User.load(provider);

    await user.shield({ amount, token });

    try {
      console.log("updating merkle tree...");
      let initLog = console.log;
      // console.log = () => {};
      await updateMerkleTreeForTest(
        provider.provider?.connection!,
        // provider.provider,
      );
      // console.log = initLog;
      console.log("✔️updated merkle tree!");
    } catch (e) {
      console.log(e);
      throw new Error("Failed to update merkle tree!");
    }
    // TODO: add random amount and amount checks
    // let balance = await user.getBalance({ latest: true });
  });

  it.only("(user class) shield SOL", async () => {
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
    // TODO: add random amount and amount checks
    try {
      console.log("updating merkle tree...");
      let initLog = console.log;
      console.log = () => {};
      await updateMerkleTreeForTest(
        provider.provider?.connection!,
        provider.provider,
      );
      console.log = initLog;
      console.log("✔️updated merkle tree!");
    } catch (e) {
      console.log(e);
      throw new Error("Failed to update merkle tree!");
    }
  });

  it("(user class) unshield SPL", async () => {
    let amount = 1;
    let token = "USDC";
    let solRecipient = SolanaKeypair.generate();
    console.log("test user wallet: ", userKeypair.publicKey.toBase58());
    const provider = await Provider.native(userKeypair); // userKeypair
    let recipientTokenAccount = await newAccountWithTokens({
      connection: provider.provider.connection,
      MINT,
      ADMIN_AUTH_KEYPAIR: userKeypair,
      userAccount: solRecipient,
      amount: new anchor.BN(0),
    });
    // console.log("recipientTokenAccount: ", recipientTokenAccount.toBase58());
    let res = await provider.provider.connection.requestAirdrop(
      userKeypair.publicKey,
      2_000_000_000,
    );

    await provider.provider.connection.confirmTransaction(res, "confirmed");
    const user = await User.load(provider);
    await user.unshield({ amount, token, recipient: solRecipient.publicKey });

    try {
      console.log("updating merkle tree...");
      let initLog = console.log;
      console.log = () => {};
      await updateMerkleTreeForTest(
        provider.provider?.connection!,
        // provider.provider,
      );
      console.log = initLog;
      console.log("✔️updated merkle tree!");
    } catch (e) {
      console.log(e);
      throw new Error("Failed to update merkle tree!");
    }
  });
  it("(user class) transfer SPL", async () => {
    let amount = 1;
    let token = "USDC";
    console.log("test user wallet: ", userKeypair.publicKey.toBase58());
    const provider = await Provider.native(userKeypair); // userKeypair
    const shieldedRecipient =
      "19a20668193c0143dd96983ef457404280741339b95695caddd0ad7919f2d434";
    const encryptionPublicKey =
      "LPx24bc92eecaf5e3904bc1f4f731a2b1e0a28adf445e800c4cff112eb7a3f5350b";

    const recipient = new anchor.BN(shieldedRecipient, "hex");
    const recipientEncryptionPublicKey: Uint8Array =
      strToArr(encryptionPublicKey);

    const user = await User.load(provider);
    await user.transfer({
      amount,
      token,
      recipient,
      recipientEncryptionPublicKey, // TODO: do shielded address
    });
    // TODO: add balance checks

    try {
      console.log("updating merkle tree...");
      let initLog = console.log;
      console.log = () => {};
      await updateMerkleTreeForTest(
        provider.provider?.connection!,
        // provider.provider,
      );
      console.log = initLog;
      console.log("✔️updated merkle tree!");
    } catch (e) {
      console.log(e);
      throw new Error("Failed to update merkle tree!");
    }
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
    const provider = await Provider.native(userKeypair);
    const user = await User.load(provider);
    await user.transfer({
      amount,
      token,
      recipient,
      recipientEncryptionPublicKey, // TODO: do shielded address
    });
    try {
      console.log("updating merkle tree...");
      let initLog = console.log;
      console.log = () => {};
      await updateMerkleTreeForTest(
        provider.provider?.connection!,
        // provider.provider,
      );
      console.log = initLog;
      console.log("✔️updated merkle tree!");
    } catch (e) {
      console.log(e);
      throw new Error("Failed to update merkle tree!");
    }

    // TODO: add random amount, recipient and amount checks
  });

  it.skip("(user class) unshield SOL", async () => {
    let amount = 1;
    let token = "SOL";
    let recipient = new PublicKey(
      "E7jqevikamCMCda8yCsfNawj57FSotUZuref9MLZpWo1",
    );
    const provider = await Provider.native(userKeypair);
    const user = await User.load(provider);
    await user.unshield({ amount, token, recipient });
    try {
      console.log("updating merkle tree...");
      let initLog = console.log;
      console.log = () => {};
      await updateMerkleTreeForTest(
        provider.provider?.connection!,
        // provider.provider,
      );
      console.log = initLog;
      console.log("✔️updated merkle tree!");
    } catch (e) {
      console.log(e);
      throw new Error("Failed to update merkle tree!");
    }
    // TODO: add random amount and amount checks
  });
});
