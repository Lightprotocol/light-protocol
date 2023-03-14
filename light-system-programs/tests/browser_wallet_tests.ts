import * as anchor from "@coral-xyz/anchor";
import { Connection, Keypair, PublicKey, SystemProgram } from "@solana/web3.js";
import _ from "lodash";

// TODO: add and use  namespaces in SDK
import {
  ADMIN_AUTH_KEYPAIR,
  MINT,
  Provider,
  newAccountWithTokens,
  User,
  strToArr,
  TOKEN_REGISTRY,
  newAccountWithTokens,
  MINT
} from "light-sdk";
import { useWallet } from "./mock/useWalletMock";
import { getAssociatedTokenAddressSync } from "@solana/spl-token";
import { assert } from "chai";

// TODO: remove deprecated function calls
describe("browser_wallet", () => {
  let connection = new Connection("http://localhost:8899/");

  const userKeypair = ADMIN_AUTH_KEYPAIR;
  
  const { signMessage, sendAndConfirmTransaction, signTransaction } = useWallet(
    userKeypair,
    connection,
  );


  it("(user class) shield SPL", async () => {
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
    );

    let res = await provider.provider.connection.requestAirdrop(
      userKeypair.publicKey,
      2_000_000_000,
    );
    // get token
    const tokenCtx = TOKEN_REGISTRY.find((t) => t.symbol === token);

    const userSplAccount = getAssociatedTokenAddressSync(
      tokenCtx!.tokenAccount,
      ADMIN_AUTH_KEYPAIR.publicKey,
    );

    const preTokenBalance =
      await provider.provider.connection.getTokenAccountBalance(userSplAccount);

    await provider.provider.connection.confirmTransaction(res, "confirmed");

    const user: User = await User.load(provider);

    const preShieldedBalance = await user.getBalance({ latest: true });

    await user.shield({ amount, token, extraSolAmount: 0 }); // 2

    // TODO: add random amount and amount checks
    await user.provider.latestMerkleTree();

    let balance;

    try {
      balance = await user.getBalance({ latest: true });
    } catch (e) {
      throw new Error(`ayayay ${e}`);
    }
    let tokenBalanceAfter = balance.find(
      (b) => b.tokenAccount.toBase58() === tokenCtx?.tokenAccount.toBase58(),
    );
    let tokenBalancePre = preShieldedBalance.find(
      (b) => b.tokenAccount.toBase58() === tokenCtx?.tokenAccount.toBase58(),
    );

    // assert that the user's shielded balance has increased by the amount shielded
    assert.equal(
      tokenBalanceAfter.amount,
      tokenBalancePre.amount + amount * tokenCtx?.decimals,
      `shielded balance after ${tokenBalanceAfter.amount} != shield amount ${
        amount * tokenCtx?.decimals
      }`,
    );

    // assert that the user's token balance has decreased by the amount shielded
    const postTokenBalance =
      await provider.provider.connection.getTokenAccountBalance(userSplAccount);
    assert.equal(
      postTokenBalance.value.uiAmount,
      preTokenBalance.value.uiAmount - amount,
      `user token balance after ${postTokenBalance.value.uiAmount} != user token balance before ${preTokenBalance.value.uiAmount} - shield amount ${amount}`,
    );

    // assert that the user's sol shielded balance has increased by the additional sol amount
    let solBalanceAfter = balance.find(
      (b) => b.tokenAccount.toBase58() === "11111111111111111111111111111111",
    );
    let solBalancePre = preShieldedBalance.find(
      (b) => b.tokenAccount.toBase58() === "11111111111111111111111111111111",
    );
    // console.log("solBalancePre", solBalancePre);
    // console.log("solBalanceAfter", solBalanceAfter);
    assert.equal(
      solBalanceAfter.amount,
      solBalancePre.amount + 150000, //+ 2 * 1e9, this MINIMZM
      `shielded sol balance after ${solBalanceAfter.amount} != shield amount 0//2 aka min sol amount (50k)`,
    );
  });

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

  it("(user class) unshield SPL", async () => {
    let amount = 1;

    let token = "USDC";
    
    let solRecipient = Keypair.generate();
    
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

    const tokenCtx = TOKEN_REGISTRY.find((t) => t.symbol === token);

    const recipientSplBalance = getAssociatedTokenAddressSync(
      tokenCtx!.tokenAccount,
      solRecipient.publicKey,
    );

    await provider.provider.connection.confirmTransaction(res, "confirmed");
    
    let preTokenBalance = { value: { uiAmount: 0 } };
    // TODO: add test case for if recipient doesnt have account yet -> relayer must create it
    await newAccountWithTokens({
      connection: provider.provider.connection,
      MINT,
      ADMIN_AUTH_KEYPAIR: userKeypair,
      userAccount: solRecipient,
      amount: new anchor.BN(0),
    });

    try {
      preTokenBalance =
        await provider.provider.connection.getTokenAccountBalance(
          recipientSplBalance,
        );
    } catch (e) {
      console.log(
        "recipient account does not exist yet (creating as part of user class)",
      );
    }

    const user = await User.load(provider);

    const preShieldedBalance = await user.getBalance({ latest: true });

    await user.unshield({ amount, token, recipient: solRecipient.publicKey });

    await user.provider.latestMerkleTree();

    let balance = await user.getBalance({ latest: true });
    let tokenBalanceAfter = balance.find(
      (b) => b.tokenAccount.toBase58() === tokenCtx?.tokenAccount.toBase58(),
    );
    let tokenBalancePre = preShieldedBalance.find(
      (b) => b.tokenAccount.toBase58() === tokenCtx?.tokenAccount.toBase58(),
    );

    // assert that the user's shielded balance has decreased by the amount unshielded
    assert.equal(
      tokenBalanceAfter.amount,
      tokenBalancePre.amount - amount * tokenCtx?.decimals, // TODO: check that fees go ?
      `shielded balance after ${tokenBalanceAfter.amount} != unshield amount ${
        amount * tokenCtx?.decimals
      }`,
    );

    // assert that the user's token balance has decreased by the amount shielded
    const postTokenBalance =
      await provider.provider.connection.getTokenAccountBalance(
        recipientSplBalance,
      );
    assert.equal(
      postTokenBalance.value.uiAmount,
      preTokenBalance.value.uiAmount + amount,
      `user token balance after ${postTokenBalance.value.uiAmount} != user token balance before ${preTokenBalance.value.uiAmount} + unshield amount ${amount}`,
    );

    // assert that the user's sol shielded balance has increased by the additional sol amount
    let solBalanceAfter = balance.find(
      (b) => b.tokenAccount.toBase58() === "11111111111111111111111111111111",
    );
    let solBalancePre = preShieldedBalance.find(
      (b) => b.tokenAccount.toBase58() === "11111111111111111111111111111111",
    );
    const minimumBalance = 150000;
    const tokenAccountFee = 500_000
    assert.equal(
      solBalanceAfter.amount,
      solBalancePre.amount - minimumBalance - tokenAccountFee, // FIXME: no fees being charged here apparently
      `shielded sol balance after ${solBalanceAfter.amount} != unshield amount ${solBalancePre.amount - minimumBalance - tokenAccountFee}`,
    );
  });

  it("(user class) transfer SPL", async () => {
    let amount = 1;

    
    const token = "USDC";
    
    const provider = await Provider.browser(
      {
        signMessage,
        signTransaction,
        sendAndConfirmTransaction,
        publicKey: userKeypair.publicKey,
      },
      connection,
    ); // userKeypair
    
    const shieldedRecipient =
      "19a20668193c0143dd96983ef457404280741339b95695caddd0ad7919f2d434";
    const encryptionPublicKey =
      "LPx24bc92eecaf5e3904bc1f4f731a2b1e0a28adf445e800c4cff112eb7a3f5350b";

    // get token from registry
    const tokenCtx = TOKEN_REGISTRY.find((t) => t.symbol === token);
    const recipient = new anchor.BN(shieldedRecipient, "hex");
    const recipientEncryptionPublicKey: Uint8Array =
      strToArr(encryptionPublicKey);

    const user = await User.load(provider);

    const preShieldedBalance = await user.getBalance({ latest: true });

    await user.transfer({
      amount,
      token,
      recipient: shieldedRecipient,
      recipientEncryptionPublicKey, // TODO: do shielded address
    });

    await user.provider.latestMerkleTree();

    let balance = await user.getBalance({ latest: true });
    let tokenBalanceAfter = balance.find(
      (b) => b.tokenAccount.toBase58() === tokenCtx?.tokenAccount.toBase58(),
    );
    let tokenBalancePre = preShieldedBalance.find(
      (b) => b.tokenAccount.toBase58() === tokenCtx?.tokenAccount.toBase58(),
    );
    // assert that the user's shielded balance has decreased by the amount transferred
    assert.equal(
      tokenBalanceAfter.amount,
      tokenBalancePre.amount - amount * tokenCtx?.decimals, // TODO: check that fees go ?
      `shielded balance after ${tokenBalanceAfter.amount} != unshield amount ${
        amount * tokenCtx?.decimals
      }`,
    );
    // assert that the user's sol shielded balance has decreased by fee
    let solBalanceAfter = balance.find(
      (b) => b.tokenAccount.toBase58() === SystemProgram.programId.toBase58(),
    );
    let solBalancePre = preShieldedBalance.find(
      (b) => b.tokenAccount.toBase58() === SystemProgram.programId.toBase58(),
    );
    const minimumChangeUtxoAmounts = 50000 * 3;
    assert.equal(
      solBalanceAfter.amount,
      solBalancePre.amount - 100000 - minimumChangeUtxoAmounts, // FIXME: no fees being charged here apparently
      `shielded sol balance after ${solBalanceAfter.amount} != unshield amount -fee -minimumSplUtxoChanges`,
    );
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
