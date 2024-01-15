import * as anchor from "@coral-xyz/anchor";
import { Keypair as SolanaKeypair, PublicKey, Keypair } from "@solana/web3.js";
import seedrandom from "seedrandom";

// Create a seeded random number generator
const rng = seedrandom("some_seed");

import {
  Provider,
  createTestAccounts,
  confirmConfig,
  User,
  TestRpc,
  Action,
  UserTestAssertHelper,
  airdropSol,
  airdropSplToAssociatedTokenAccount,
  convertAndComputeDecimals,
  TOKEN_REGISTRY,
  ADMIN_AUTH_KEYPAIR,
  RPC_FEE,
  BN_0,
} from "../../src";

import { BN } from "@coral-xyz/anchor";
import {
  transfer as splTransfer,
  getAccount,
  getAssociatedTokenAddressSync,
  createAssociatedTokenAccount,
} from "@solana/spl-token";
import { assert } from "chai";
import { WasmFactory } from "@lightprotocol/account.rs";

function generateRandomTestAmount(
  min: number = 0.2,
  max: number = 2,
  decimals: number,
  rng: any,
): number {
  if (min > max) throw new Error(`min ${min} must be less than max ${max}`);
  const randomAmount = rng() * (max - min) + min;
  return +randomAmount.toFixed(decimals);
}

const getRandomObject = <T>(arr: T[]): T | null =>
  arr.length ? arr[Math.floor(rng() * arr.length)] : null;
const getRandomObjectAndArray = <T>(arr: T[]): [T | null, T[]] => {
  if (arr.length === 0) {
    return [null, arr];
  }

  const index = Math.floor(rng() * arr.length);
  const selectedObject = arr[index];
  const newArray = [...arr.slice(0, index), ...arr.slice(index + 1)];

  return [selectedObject, newArray];
};

const calculateAmounts = async (
  user: User,
  rng: any,
  sol: boolean,
  isSol: boolean,
  mint: PublicKey,
) => {
  const balance = await user.getBalance();

  const rpcFee = user.provider.rpc.getRpcFee().toNumber();
  let totalSolBalance = balance.tokenBalances.get(mint.toBase58())
    ? balance.tokenBalances.get(mint.toBase58())?.totalBalanceSol.toNumber()
    : 0;
  totalSolBalance = totalSolBalance ?? 0;

  let totalSplBalance =
    balance.tokenBalances.get(mint.toBase58()) && !isSol
      ? balance.tokenBalances.get(mint.toBase58())?.totalBalanceSpl.toNumber()
      : 0;
  totalSplBalance = totalSplBalance ?? 0;

  const amountSol =
    isSol || sol
      ? generateRandomTestAmount(0.01, (totalSolBalance - rpcFee) / 1e9, 9, rng)
      : undefined;
  const amountSpl = !isSol
    ? generateRandomTestAmount(0.01, totalSplBalance / 1e2, 2, rng)
    : undefined;

  return { amountSol, amountSpl };
};
const shield = async (
  user: User,
  rng: any,
  sol: boolean,
  token: string = "SOL",
  keypair: Keypair,
) => {
  const isSol = token === "SOL";

  const testInputs = {
    amountSol: sol ? generateRandomTestAmount(1, 5000, 9, rng) : undefined,
    amountSpl: !isSol ? generateRandomTestAmount(1, 5000, 2, rng) : undefined,
    token,
    type: Action.SHIELD,
    expectedUtxoHistoryLength: 1,
  };
  console.log(
    "shielding ",
    testInputs.amountSol,
    " ",
    testInputs.amountSpl,
    " ",
    testInputs.token,
  );

  const userTestAssertHelper = new UserTestAssertHelper({
    userSender: user,
    userRecipient: user,
    provider: user.provider,
    testInputs,
  });
  await airdropSol({
    connection: user.provider.provider.connection,
    lamports:
      convertAndComputeDecimals(testInputs.amountSol!, new BN(1e9)).toNumber() +
      5000,
    recipientPublicKey: user.provider.wallet.publicKey,
  });
  if (!isSol) {
    await airdropSplToAssociatedTokenAccount(
      user.provider.provider.connection,
      convertAndComputeDecimals(testInputs.amountSpl!, new BN(1e2)).toNumber(),
      keypair.publicKey,
    );
  }
  await userTestAssertHelper.fetchAndSaveState();

  const res = await user.shield({
    publicAmountSol: testInputs.amountSol,
    publicAmountSpl: testInputs.amountSpl,
    token: testInputs.token,
  });
  console.log(res);

  if (isSol) {
    await userTestAssertHelper.checkSolShielded();
  } else {
    await userTestAssertHelper.checkSplShielded();
  }
};

const unshield = async (
  user: User,
  rng: any,
  sol: boolean,
  token: string = "SOL",
) => {
  const isSol = token === "SOL";
  const mint = TOKEN_REGISTRY.get(token)!.mint;
  const { amountSol, amountSpl } = await calculateAmounts(
    user,
    rng,
    sol,
    isSol,
    mint,
  );
  const recipientKeypair = SolanaKeypair.generate();

  const testInputs = {
    token,
    type: Action.UNSHIELD,
    expectedUtxoHistoryLength: 0,
    amountSol,
    amountSpl,
    recipient: recipientKeypair.publicKey,
  };
  console.log(
    "unshielding ",
    testInputs.amountSol,
    " ",
    testInputs.amountSpl,
    " to ",
    testInputs.recipient,
  );
  const userTestAssertHelper = new UserTestAssertHelper({
    userSender: user,
    userRecipient: user,
    provider: user.provider,
    testInputs,
  });
  await userTestAssertHelper.fetchAndSaveState();
  const res = await user.unshield({
    publicAmountSol: testInputs.amountSol,
    publicAmountSpl: testInputs.amountSpl,
    token,
    recipient: testInputs.recipient,
  });
  console.log(res);
  if (isSol) {
    await userTestAssertHelper.checkSolUnshielded();
  } else {
    await userTestAssertHelper.checkSplUnshielded();
    const recipientAssociatedTokenAccount = getAssociatedTokenAddressSync(
      mint,
      testInputs.recipient,
    );
    const splRecipientTokenAccount = await getAccount(
      user.provider.provider.connection,
      recipientAssociatedTokenAccount,
      "processed",
    );

    const splTransferRecipientKeypair = SolanaKeypair.generate();
    await airdropSol({
      connection: user.provider.provider.connection,
      lamports: 1000000000,
      recipientPublicKey: splTransferRecipientKeypair.publicKey,
    });
    const splTransferRecipient = getAssociatedTokenAddressSync(
      mint,
      splTransferRecipientKeypair.publicKey,
    );
    const createSplTransferRecipientTx = await createAssociatedTokenAccount(
      user.provider.provider.connection,
      splTransferRecipientKeypair,
      mint,
      splTransferRecipientKeypair.publicKey,
    );
    console.log(
      "createSplTransferRecipientTx tx after unshield: ",
      createSplTransferRecipientTx,
    );

    const transferTx = await splTransfer(
      user.provider.provider.connection,
      recipientKeypair,
      recipientAssociatedTokenAccount,
      splTransferRecipient,
      recipientKeypair.publicKey,
      splRecipientTokenAccount.amount,
      [],
      {
        skipPreflight: true,
        commitment: "confirmed",
      },
    );
    console.log("spl transfer tx after unshield: ", transferTx);
    const splTransferRecipientTokenAccount = await getAccount(
      user.provider.provider.connection,
      splTransferRecipient,
      "processed",
    );
    assert.equal(
      splTransferRecipientTokenAccount.amount,
      splRecipientTokenAccount.amount,
      `spl transfer recipient token account amount should be ${splRecipientTokenAccount.amount} but is ${splTransferRecipientTokenAccount.amount}`,
    );
  }
};

const transfer = async (
  senderUser: User,
  recipientUser: User,
  rng: any,
  sol: boolean,
  token: string = "SOL",
) => {
  const isSol = token === "SOL";
  const mint = TOKEN_REGISTRY.get(token)!.mint;
  const { amountSol, amountSpl } = await calculateAmounts(
    senderUser,
    rng,
    sol,
    isSol,
    mint,
  );

  const recipientInboxBalance = (
    await recipientUser.getUtxoInbox()
  ).tokenBalances.get(TOKEN_REGISTRY.get(token)!.mint.toBase58());

  const testInputs = {
    token,
    type: Action.TRANSFER,
    expectedUtxoHistoryLength: 0,
    amountSol,
    amountSpl,
    recipientUser,
    expectedRecipientUtxoLength: recipientInboxBalance
      ? recipientInboxBalance.utxos.size + 1
      : 1,
  };

  console.log("transferring ", testInputs.amountSol);
  const userTestAssertHelper = new UserTestAssertHelper({
    userSender: senderUser,
    userRecipient: recipientUser,
    provider: senderUser.provider,
    testInputs,
  });
  await userTestAssertHelper.fetchAndSaveState();

  const res = await senderUser.transfer({
    amountSol: testInputs.amountSol,
    amountSpl: testInputs.amountSpl,
    token,
    recipient: recipientUser.account.getPublicKey(),
  });
  console.log(res);
  if (isSol) {
    await userTestAssertHelper.checkSolTransferred();
  } else {
    await userTestAssertHelper.checkSplTransferred();
  }
};

const mergeAllInboxUtxos = async (user: User, token: string) => {
  const tokenCtx = TOKEN_REGISTRY.get(token)!;
  const inboxBalance = await user.getUtxoInbox();
  const tokenUtxoBalances = inboxBalance.tokenBalances.get(
    tokenCtx.mint.toBase58(),
  );
  if (
    tokenUtxoBalances &&
    (tokenUtxoBalances.totalBalanceSol.toNumber() > 0 ||
      tokenUtxoBalances?.totalBalanceSpl.toNumber() > 0)
  ) {
    console.log("merging all utxos for ", token);
    const userTestAssertHelper = new UserTestAssertHelper({
      userSender: user,
      userRecipient: user,
      provider: user.provider,
      testInputs: {
        token,
        type: Action.TRANSFER,
        expectedUtxoHistoryLength: 0,
        isMerge: true,
      },
    });
    await userTestAssertHelper.fetchAndSaveState();
    await user.mergeAllUtxos(tokenCtx.mint);
    await userTestAssertHelper.checkMergedAll();
    console.log(`merged all ${token} utxos`);
  }
};

const createTestUser = async (
  anchorProvider: anchor.AnchorProvider,
  rpc: TestRpc,
): Promise<{ user: User; wallet: Keypair }> => {
  const wallet = SolanaKeypair.generate();
  await airdropSol({
    connection: anchorProvider.connection,
    lamports: 1e9,
    recipientPublicKey: wallet.publicKey,
  });
  const provider = await Provider.init({
    wallet,
    rpc,
    confirmConfig,
  });
  return { user: await User.init({ provider }), wallet };
};

const checkSolBalanceGtRpcFee = async (user: User, tokenMint: PublicKey) => {
  const balance = await user.getBalance();
  const solBalance = balance.tokenBalances.get(tokenMint.toBase58())
    ?.totalBalanceSol;
  const splBalance = balance.tokenBalances.get(tokenMint.toBase58())
    ?.totalBalanceSpl;
  if (
    tokenMint.toBase58() !== PublicKey.default.toBase58() &&
    (!splBalance || splBalance.eq(BN_0))
  )
    return false;
  return (
    solBalance &&
    solBalance.toNumber() > user.provider.rpc.getRpcFee().toNumber() + 1e9
  );
};

const selectRandomAction = async (
  rng: any,
  user: User,
  tokenMint: PublicKey,
) => {
  const randomNumber = rng();
  const unshieldingPossible = await checkSolBalanceGtRpcFee(user, tokenMint);
  console.log("unshielding possible ", unshieldingPossible);

  if (randomNumber < 0.33 && unshieldingPossible) {
    return Action.TRANSFER;
  } else if (randomNumber < 0.66 && unshieldingPossible) {
    return Action.UNSHIELD;
  }
  return Action.SHIELD;
};

describe("Test User", () => {
  // Configure the client to use the local cluster.
  process.env.ANCHOR_WALLET = process.env.HOME + "/.config/solana/id.json";
  process.env.ANCHOR_PROVIDER_URL = "http://127.0.0.1:8899";

  const anchorProvider = anchor.AnchorProvider.local(
    "http://127.0.0.1:8899",
    confirmConfig,
  );
  anchor.setProvider(anchorProvider);

  /**
   * performs random tests with deterministic testing randomness
   *
   * @param noUsers
   * @param anchorProvider
   */
  async function randomTest(
    noUsers: number,
    anchorProvider: anchor.AnchorProvider,
    token: "SOL" | "USDC" | "BOTH" = "SOL",
  ) {
    await createTestAccounts(anchorProvider.connection);

    const rpcKeypair = SolanaKeypair.generate();
    const rpcRecipientSol = SolanaKeypair.generate();

    // funding rpc
    await airdropSol({
      connection: anchorProvider.connection,
      lamports: 1e9,
      recipientPublicKey: rpcKeypair.publicKey,
    });
    // funding rpc rpcRecipientSol
    await airdropSol({
      connection: anchorProvider.connection,
      lamports: 1e9,
      recipientPublicKey: rpcRecipientSol.publicKey,
    });

    const rpc = new TestRpc({
      rpcPubkey: ADMIN_AUTH_KEYPAIR.publicKey,
      rpcRecipientSol: rpcRecipientSol.publicKey,
      rpcFee: RPC_FEE,
      payer: ADMIN_AUTH_KEYPAIR,
      connection: anchorProvider.connection,
      lightWasm: await WasmFactory.getInstance(),
    });

    const testUsers: { user: User; wallet: Keypair }[] = [];
    for (let user = 0; user < noUsers; user++) {
      testUsers.push(await createTestUser(anchorProvider, rpc));
    }

    /**
     * select user
     * select action
     * perform action
     */
    let transactions = 0;
    // eslint-disable-next-line no-constant-condition
    while (true) {
      console.log("\n----------------------------------\n");
      const res = getRandomObjectAndArray(testUsers);
      const rndUser = res[0]!.user;
      const wallet = res[0]!.wallet;
      const remainingTestUsers = res[1];
      const _token =
        token != "BOTH" ? token : getRandomObject(["SOL", "USDC"])!;
      const tokenMint = TOKEN_REGISTRY.get(_token)!.mint;
      console.log("no users ", testUsers.length);
      console.log("no transactions ", transactions);
      await rndUser.getBalance();
      const randomAction = await selectRandomAction(rng, rndUser, tokenMint);
      console.log("random action ", randomAction);
      console.log("selected user ", rndUser.account.getPublicKey());
      console.log("selected _token ", _token);

      try {
        if (randomAction === Action.SHIELD) {
          await mergeAllInboxUtxos(rndUser, _token);
          await shield(rndUser, rng, true, _token, wallet);
        } else if (randomAction === Action.TRANSFER) {
          await mergeAllInboxUtxos(rndUser, _token);
          const createNewUser = rng();
          let recipientUser: { user: User; wallet: Keypair };
          if (remainingTestUsers.length == 0 || createNewUser < 0.5 / noUsers) {
            recipientUser = await createTestUser(anchorProvider, rpc);
            testUsers.push(recipientUser);
          } else {
            recipientUser = getRandomObject(remainingTestUsers)!;
          }
          console.log(
            "selected recipientUser ",
            recipientUser.user.account.getPublicKey(),
          );

          await transfer(rndUser, recipientUser.user, rng, true, _token);
        } else {
          await mergeAllInboxUtxos(rndUser, _token);
          await unshield(rndUser, rng, true, _token);
        }
        transactions++;
      } catch (error) {
        const inboxBalance = await rndUser.getUtxoInbox();
        console.log("inboxBalance ", inboxBalance);
        const balance = await rndUser.getBalance();
        console.log("balance ", balance);
        throw error;
      }
    }
  }

  it("random sol test", async () => {
    await randomTest(1, anchorProvider);
  });
  it("random spl test", async () => {
    await randomTest(1, anchorProvider, "USDC");
  });

  /**
   * History is still buggy
   * fails at tx 22
   * - had two compressions before one usdc and one sol
   * - history only finds the latest sol compression
   *
   * Check:
   * - that usdc transactions are categorized correctly
   * */
  it.skip("random sol & spl test", async () => {
    await randomTest(1, anchorProvider, "BOTH");
  });
});
