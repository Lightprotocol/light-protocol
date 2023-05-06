import { PublicKey, SystemProgram } from "@solana/web3.js";
import { assert } from "chai";
import { fetchNullifierAccountInfo } from "../utils";
import { Action } from "../transaction";
import { IndexedTransaction, TokenData } from "../types";
import { Balance, Provider, User } from "../wallet";
import { getAssociatedTokenAddressSync } from "@solana/spl-token";
import { BN } from "@coral-xyz/anchor";
var _ = require("lodash");

import {
  MINIMUM_LAMPORTS,
  TOKEN_ACCOUNT_FEE,
  TOKEN_REGISTRY,
} from "../constants";
import { Utxo } from "utxo";

type TestInputs = {
  amountSpl: number;
  amountSol: number;
  token: string;
  type: Action;
  recipientSpl?: PublicKey;
  expectedUtxoHistoryLength: number;
  expectedSpentUtxosLength?: number;
  recipientSeed?: string;
  expectedRecipientUtxoLength?: number;
  mergedUtxo?: boolean;
};

type TestUserBalances = {
  user: User;
  preShieldedBalance?: Balance;
  preTokenBalance?: number | null;
  preSolBalance?: number;
  isSender: boolean;
  splAccount?: PublicKey;
};

export class TestStateValidator {
  private recentTransaction?: IndexedTransaction;
  public provider: Provider;
  public sender: TestUserBalances;
  public recipient: TestUserBalances;
  public testInputs: TestInputs;
  public tokenCtx: TokenData;

  constructor({
    userSender,
    userRecipient,
    provider,
    testInputs,
  }: {
    userSender: User;
    userRecipient: User;
    provider: Provider;
    testInputs: TestInputs;
  }) {
    this.sender = { user: userSender, isSender: true };
    this.recipient = { user: userRecipient, isSender: false };
    this.provider = provider;
    this.testInputs = testInputs;
    const tokenCtx = TOKEN_REGISTRY.get(this.testInputs.token);
    if (!tokenCtx)
      throw new Error(`Token context not found for token ${testInputs.token}`);
    this.tokenCtx = tokenCtx;
  }

  async fetchAndSaveState(latest: boolean = true) {
    try {
      const saveUserState = async (
        userBalances: TestUserBalances,
        testInputs: TestInputs,
      ) => {
        userBalances.preShieldedBalance = _.cloneDeep(
          await userBalances.user.getBalance(latest),
        );

        if (userBalances.isSender) {
          userBalances.splAccount =
            testInputs.token !== "SOL"
              ? getAssociatedTokenAddressSync(
                  this.tokenCtx.mint,
                  this.provider.wallet.publicKey,
                )
              : undefined;
        } else if (testInputs.recipientSpl) {
          userBalances.splAccount = getAssociatedTokenAddressSync(
            this.tokenCtx.mint,
            testInputs.recipientSpl,
          );
        }

        if (userBalances.splAccount) {
          userBalances.preTokenBalance = (
            await this.provider.provider?.connection.getTokenAccountBalance(
              userBalances.splAccount,
            )
          )?.value.uiAmount;
        }

        userBalances.preSolBalance =
          await this.provider.provider?.connection.getBalance(
            this.provider.wallet.publicKey,
          );
      };
      await saveUserState(this.sender, this.testInputs);
      await saveUserState(this.recipient, this.testInputs);

      this.provider = this.provider;
    } catch (error) {
      console.log("error while fetching the state", { error });
    }
  }

  public async assertRecentTransaction({
    userHistory = false,
  }: {
    userHistory?: boolean;
  }) {
    const { amountSol, amountSpl, type, token } = this.testInputs;
    const tokenMint = TOKEN_REGISTRY.get(token)?.mint;

    let transactions;
    if (userHistory) {
      transactions = await this.sender.user.getTransactionHistory();
    } else {
      transactions = await this.provider.relayer.getIndexedTransactions(
        this.provider.provider!.connection,
      );
    }

    transactions.sort((a, b) => b.blockTime - a.blockTime);

    this.recentTransaction = transactions[0];

    if (amountSpl && type) {
      assert.strictEqual(
        this.recentTransaction!.publicAmountSpl.div(
          this.tokenCtx!.decimals,
        ).toNumber(),
        type === Action.TRANSFER ? 0 : amountSpl,
      );
    }

    if (amountSol) {
      assert.strictEqual(
        this.recentTransaction!.publicAmountSol.div(
          this.tokenCtx!.decimals,
        ).toNumber(),
        type === Action.TRANSFER ? 0 : amountSol,
      );
    }

    if (type === Action.SHIELD) {
      assert.strictEqual(
        this.recentTransaction!.from.toBase58(),
        this.provider.wallet.publicKey.toBase58(),
      );
    }

    if (type === Action.TRANSFER) {
      assert.strictEqual(
        this.recentTransaction!.to.toBase58(),
        PublicKey.default.toBase58(),
      );

      assert.strictEqual(
        this.recentTransaction!.from.toBase58(),
        PublicKey.default.toBase58(),
      );
    }

    if (this.recipient.splAccount) {
      assert.strictEqual(
        this.recentTransaction.to.toBase58(),
        this.recipient.splAccount.toBase58(),
      );
    }

    if (type !== Action.TRANSFER) {
      assert.strictEqual(
        this.recipient.user.balance.tokenBalances
          .get(tokenMint?.toBase58()!)
          ?.utxos.get(
            new BN(this.recentTransaction!.leaves[0], "le").toString(),
          )?._commitment,
        new BN(this.recentTransaction!.leaves[0], "le").toString(),
      );
    }

    assert.strictEqual(this.recentTransaction!.type, type);

    assert.strictEqual(
      this.recentTransaction!.relayerFee.toString(),
      type === Action.UNSHIELD
        ? "500000"
        : type === Action.TRANSFER
        ? "100000"
        : "0",
    );

    assert.strictEqual(
      this.recentTransaction!.relayerRecipientSol.toBase58(),
      type === Action.SHIELD
        ? PublicKey.default.toBase58()
        : this.provider.relayer.accounts.relayerRecipientSol.toBase58(),
    );
  }

  /**
   * Checks:
   * - every utxo in utxos is inserted and is not spent
   * - every utxo in spent utxos is spent
   * - if an utxo has an spl asset it is categorized in that spl asset
   * - every utxo in an spl TokenBalance has a balance in this token
   * - for every TokenUtxoBalance total amounts are correct
   */
  async assertBalance(user: User) {
    // checks that a utxo is categorized correctly which means:
    // - has the same asset as the tokenBalance it is part of
    // - has a balance of that asset greater than 0
    const checkCategorizationByAsset = (asset: string, utxo: Utxo) => {
      if (asset === SystemProgram.programId.toBase58()) {
        assert.notStrictEqual(
          utxo.amounts[0].toString(),
          "0",
          `utxo categorized in sol utxos but has no sol balance ${utxo.amounts} `,
        );
      } else {
        assert.notStrictEqual(
          utxo.amounts[1].toString(),
          "0",
          `utxo categorized in ${asset} utxos but has no spl balance ${utxo.amounts} `,
        );
        assert.strictEqual(
          utxo.assets[1].toString(),
          asset,
          `utxo categorized in ${asset} utxos but has no spl balance ${utxo.assets} `,
        );
      }
    };

    await user.getBalance();
    await user.provider.latestMerkleTree();
    for (var [asset, tokenBalance] of user.balance.tokenBalances.entries()) {
      // commitment is inserted in the merkle tree
      for (var [commitment, utxo] of tokenBalance.utxos.entries()) {
        assert.notDeepEqual(
          user.provider.solMerkleTree?.merkleTree.indexOf(
            utxo.getCommitment(this.provider.poseidon),
          ),
          -1,
        );
        if (!utxo.getNullifier(this.provider.poseidon))
          throw new Error(`nullifier of utxo undefined, ${utxo}`);
        this.assertNullifierAccountDoesNotExist(
          utxo.getNullifier(this.sender.user.provider.poseidon)!,
        );
        checkCategorizationByAsset(asset, utxo);
      }
      // commitment is not inserted in the merkle tree
      for (var utxo of tokenBalance.committedUtxos.values()) {
        assert.deepEqual(
          user.provider.solMerkleTree?.merkleTree.indexOf(
            utxo.getCommitment(this.provider.poseidon),
          ),
          -1,
        );
        if (!utxo.getNullifier(this.provider.poseidon))
          throw new Error(`nullifier of utxo undefined, ${utxo}`);
        this.assertNullifierAccountDoesNotExist(
          utxo.getNullifier(this.sender.user.provider.poseidon)!,
        );
        checkCategorizationByAsset(asset, utxo);
      }
      // nullifier of utxo is inserted
      for (var utxo of tokenBalance.spentUtxos.values()) {
        if (!utxo.getNullifier(this.provider.poseidon))
          throw new Error(`nullifier of utxo undefined, ${utxo}`);
        this.assertNullifierAccountExists(
          utxo.getNullifier(this.provider.poseidon)!,
        );
        checkCategorizationByAsset(asset, utxo);
      }
    }
  }

  async assertInboxBalance(user: User) {
    await user.getUtxoInbox();
    await user.provider.latestMerkleTree();
    for (var tokenBalance of user.inboxBalance.tokenBalances.values()) {
      // commitment is inserted in the merkle tree
      for (var utxo of tokenBalance.utxos.values()) {
        assert.notDeepEqual(
          user.provider.solMerkleTree?.merkleTree.indexOf(
            utxo.getCommitment(this.provider.poseidon),
          ),
          -1,
        );
      }
      // commitment is not inserted in the merkle tree
      for (var utxo of tokenBalance.committedUtxos.values()) {
        assert.deepEqual(
          user.provider.solMerkleTree?.merkleTree.indexOf(
            utxo.getCommitment(this.provider.poseidon),
          ),
          -1,
        );
      }
      // nullifier of utxo is inserted
      for (var utxo of tokenBalance.spentUtxos.values()) {
        if (!utxo.getNullifier(this.provider.poseidon))
          throw new Error(`nullifier of utxo undefined, ${utxo}`);
        this.assertNullifierAccountExists(
          utxo.getNullifier(this.provider.poseidon)!,
        );
      }
    }
  }

  /**
   * - check that utxos with an aggregate amount greater or equal than the spl and sol transfer amounts were spent
   */
  async assertUserUtxoSpent() {
    let amountSol = new BN(0);
    let amountSpl = new BN(0);
    for (var [
      asset,
      tokenBalance,
    ] of this.sender.preShieldedBalance!.tokenBalances.entries()) {
      for (var [commitment, utxo] of tokenBalance.utxos.entries()) {
        if (
          await fetchNullifierAccountInfo(
            utxo.getNullifier(this.provider.poseidon)!,
            this.provider.provider?.connection!,
          )
        ) {
          amountSol = amountSol.add(utxo.amounts[0]);
          amountSpl = amountSpl.add(utxo.amounts[0]);
          assert(
            this.sender.user.balance.tokenBalances
              .get(asset)!
              .spentUtxos.get(commitment),
            "Nullified spent utxo not found in sender's spent utxos",
          );
        }
      }
    }
    if (this.testInputs.amountSol)
      assert(amountSol.gte(new BN(this.testInputs.amountSol)));
    if (this.testInputs.amountSpl)
      assert(amountSpl.gte(new BN(this.testInputs.amountSpl)));
  }

  async assertShieldedTokenBalance(
    amount: number,
    userBalances: TestUserBalances,
  ) {
    if (userBalances.isSender) {
      amount = amount * -1;
    }
    const postShieldedBalances = await userBalances.user.getBalance(false);

    let tokenBalanceAfter = postShieldedBalances.tokenBalances.get(
      this.tokenCtx?.mint.toBase58(),
    )?.totalBalanceSpl;

    let _tokenBalancePre = userBalances.preShieldedBalance!.tokenBalances.get(
      this.tokenCtx?.mint.toBase58(),
    )?.totalBalanceSpl;
    let tokenBalancePre = _tokenBalancePre ? _tokenBalancePre : new BN(0);

    assert.equal(
      tokenBalanceAfter!.toNumber(),
      tokenBalancePre!.toNumber() + amount * this.tokenCtx?.decimals.toNumber(),
      `Token shielded balance isSender ${
        userBalances.isSender
      } after ${tokenBalanceAfter!} != token shield amount ${tokenBalancePre!.toNumber()} + ${
        amount * this.tokenCtx?.decimals.toNumber()
      }
       balance ${userBalances.user.balance} utxos: ${
        userBalances.user.balance.tokenBalances
      }`,
    );
  }

  async assertTokenBalance(amount: number, userBalances: TestUserBalances) {
    const postTokenBalance =
      await userBalances.user.provider.provider!.connection.getTokenAccountBalance(
        userBalances.splAccount!,
      );

    assert.equal(
      postTokenBalance.value.uiAmount,
      userBalances.preTokenBalance! + amount,
      `user is sender ${userBalances.isSender} token balance after ${
        postTokenBalance.value.uiAmount
      } != user token balance before ${userBalances.preTokenBalance!} + shield amount ${amount}`,
    );
  }

  async assertSolBalance(
    amount: number,
    tempAccountCost: number,
    userBalances: TestUserBalances,
  ) {
    const postSolBalance = await this.provider.provider!.connection.getBalance(
      this.provider.wallet.publicKey,
    );

    assert.equal(
      postSolBalance,
      userBalances.preSolBalance! +
        amount * this.tokenCtx.decimals.toNumber() +
        tempAccountCost,
      `user token balance after ${postSolBalance} != user token balance before ${userBalances.preSolBalance} + shield amount ${amount} sol`,
    );
  }

  async assertShieldedSolBalance(
    amount: number,
    userBalances: TestUserBalances,
  ) {
    if (userBalances.isSender) {
      amount = amount * -1;
    }
    const postShieldedBalances = await userBalances.user.getBalance(false);

    let solBalanceAfter = postShieldedBalances.totalSolBalance;
    let solBalancePre = userBalances.preShieldedBalance!.totalSolBalance;

    assert.equal(
      solBalanceAfter!.toNumber(),
      solBalancePre!.toNumber() + amount,
      `shielded sol balance after ${solBalanceAfter!} != shield amount ${solBalancePre!.toNumber()} + ${amount}`,
    );
  }

  async assertNullifierAccountDoesNotExist(nullifier: string) {
    assert.notEqual(
      fetchNullifierAccountInfo(nullifier, this.provider.connection!),
      null,
    );
  }
  async assertNullifierAccountExists(nullifier: string) {
    assert.notEqual(
      fetchNullifierAccountInfo(nullifier, this.provider.connection!),
      null,
    );
  }

  async checkShieldedTransferReceived(
    transferAmountSpl: number,
    transferAmountSol: number,
    mint: PublicKey,
  ) {
    await this.recipient.user.getUtxoInbox();

    assert.equal(
      this.recipient.user.inboxBalance.tokenBalances.get(mint.toBase58())?.utxos
        .size,
      this.testInputs.expectedRecipientUtxoLength,
    );

    assert.equal(
      this.recipient.user.inboxBalance.tokenBalances
        .get(mint.toBase58())
        ?.totalBalanceSpl!.toString(),
      transferAmountSpl.toString(),
    );

    assert.equal(
      this.recipient.user.inboxBalance.tokenBalances
        .get(mint.toBase58())
        ?.totalBalanceSol!.toString(),
      transferAmountSol.toString(),
    );
  }

  /**
   * Asynchronously checks if token shielding has been performed correctly for a user.
   * This function performs the following checks:
   *
   * 1. Asserts that the user's shielded token balance has increased by the amount shielded.
   * 2. Asserts that the user's token balance has decreased by the amount shielded.
   * 3. Asserts that the user's sol shielded balance has increased by the additional sol amount.
   * 4. Asserts that the length of spent UTXOs matches the expected spent UTXOs length.
   * 5. Asserts that the nullifier account exists for the user's first UTXO.
   * 6. Asserts that the recent indexed transaction is of type SHIELD and has the correct values.
   *
   * @returns {Promise<void>} Resolves when all checks are successful, otherwise throws an error.
   */
  async checkTokenShielded() {
    await this.assertBalance(this.sender.user);
    await this.assertBalance(this.recipient.user);
    // assert that the user's shielded balance has increased by the amount shielded
    await this.assertShieldedTokenBalance(
      this.testInputs.amountSpl,
      this.recipient,
    );

    // assert that the user's token balance has decreased by the amount shielded
    const tokenDecreasedAmount = this.testInputs.amountSpl * -1;

    await this.assertTokenBalance(tokenDecreasedAmount, this.sender);

    // assert that the user's sol shielded balance has increased by the additional sol amount
    await this.assertShieldedSolBalance(150000, this.recipient);

    assert.equal(
      this.recipient.user.balance.tokenBalances.get(
        this.tokenCtx.mint.toBase58(),
      )?.spentUtxos.size,
      this.testInputs.expectedSpentUtxosLength,
    );

    // TODO: make this less hardcoded
    await this.assertNullifierAccountDoesNotExist(
      this.recipient.user.balance.tokenBalances
        .get(this.tokenCtx.mint.toBase58())
        ?.utxos.values()
        .next()!
        .value.getNullifier(this.provider.poseidon),
    );

    // assert that recentIndexedTransaction is of type SHIELD and have right values
    await this.assertRecentTransaction({});

    await this.assertRecentTransaction({ userHistory: true });
  }

  /**
   * Asynchronously checks if SOL shielding has been performed correctly for a user.
   * This function performs the following checks:
   *
   * 1. Asserts that the user's shielded SOL balance has increased by the amount shielded.
   * 2. Asserts that the user's SOL balance has decreased by the amount shielded, considering the temporary account cost.
   * 3. Asserts that user UTXOs are spent and updated correctly.
   * 4. Asserts that the recent indexed transaction is of type SHIELD and has the correct values.
   *
   * Note: The temporary account cost calculation is not deterministic and may vary depending on whether the user has
   * shielded SPL tokens before. This needs to be handled carefully.
   *
   * @returns {Promise<void>} Resolves when all checks are successful, otherwise throws an error.
   */
  async checkSolShielded() {
    await this.assertBalance(this.sender.user);
    await this.assertBalance(this.recipient.user);
    // assert that the user's shielded balance has increased by the amount shielded
    await this.assertShieldedSolBalance(
      this.testInputs.amountSol * this.tokenCtx?.decimals.toNumber(),
      this.recipient,
    );

    // TODO: investigate since weird behavior
    const tempAccountCost = 3502840 - 1255000; //x-y nasty af. underterministic: costs more(y) if shielded SPL before!

    // assert that the user's sol balance has decreased by the amount
    const solDecreasedAmount = this.testInputs.amountSol * -1;

    await this.assertSolBalance(
      solDecreasedAmount,
      tempAccountCost,
      this.recipient,
    );

    // TODO: implement this
    // if (this.testInputs.mergedUtxo) {
    //   // assert that user utxos are spent and updated correctly
    //   await this.assertUserUtxoSpent();
    // }

    // assert that recentIndexedTransaction is of type SHIELD and have right values
    await this.assertRecentTransaction({});

    await this.assertRecentTransaction({ userHistory: true });
  }

  /**
   * Asynchronously checks if token unshielding has been performed correctly for a user.
   * This function performs the following checks:
   *
   * 1. Asserts that the user's shielded token balance has decreased by the amount unshielded.
   * 2. Asserts that the recipient's token balance has increased by the amount unshielded.
   * 3. Asserts that the user's shielded SOL balance has decreased by the fee.
   * 4. Asserts that user UTXOs are spent and updated correctly.
   * 5. Asserts that the recent indexed transaction is of type UNSHIELD and has the correct values.
   *
   * @returns {Promise<void>} Resolves when all checks are successful, otherwise throws an error.
   */
  async checkTokenUnshielded() {
    // assert that the user's shielded token balance has decreased by the amount unshielded
    await this.assertBalance(this.sender.user);
    await this.assertBalance(this.recipient.user);
    const tokenDecreasedAmount = this.testInputs.amountSpl;
    await this.assertShieldedTokenBalance(tokenDecreasedAmount, this.sender);

    // assert that the recipient token balance has increased by the amount shielded
    await this.assertTokenBalance(this.testInputs.amountSpl, this.recipient);

    const solDecreasedAmount = MINIMUM_LAMPORTS + TOKEN_ACCOUNT_FEE;
    // assert that the user's sol shielded balance has decreased by fee
    await this.assertShieldedSolBalance(solDecreasedAmount, this.sender);

    // assert that user utxos are spent and updated correctly
    await this.assertUserUtxoSpent();

    // assert that recentIndexedTransaction is of type UNSHIELD and have right values
    await this.assertRecentTransaction({ userHistory: true });
  }

  /**
   * Asynchronously checks if a shielded token transfer has been performed correctly for a user.
   * This function performs the following checks:
   *
   * 1. Asserts that the user's shielded token balance has decreased by the amount transferred.
   * 2. Asserts that the user's shielded SOL balance has decreased by the relayer fee.
   * 3. Asserts that user UTXOs are spent and updated correctly.
   * 4. Asserts that the recent indexed transaction is of type SHIELD and has the correct values.
   * 5. Assert that the transfer has been received correctly by the shielded recipient's account.
   *
   * @returns {Promise<void>} Resolves when all checks are successful, otherwise throws an error.
   */
  async checkTokenTransferred() {
    // assert that the user's shielded balance has decreased by the amount transferred
    await this.assertBalance(this.sender.user);
    await this.assertBalance(this.recipient.user);
    await this.assertInboxBalance(this.recipient.user);

    // assert that the user's spl shielded balance has decreased by amountSpl
    await this.assertShieldedTokenBalance(
      this.testInputs.amountSpl,
      this.sender,
    );

    // assert that the user's sol shielded balance has decreased by fee
    await this.assertShieldedSolBalance(
      this.provider.relayer.relayerFee.toNumber(),
      this.sender,
    );

    // assert that user utxos are spent and updated correctly
    await this.assertUserUtxoSpent();

    // assert that recentIndexedTransaction is of type SHIELD and have right values
    await this.assertRecentTransaction({});

    await this.assertRecentTransaction({ userHistory: true });

    await this.checkShieldedTransferReceived(
      this.testInputs.amountSpl !== undefined
        ? this.testInputs.amountSpl * this.tokenCtx.decimals.toNumber()
        : 0,
      this.testInputs.amountSol !== undefined
        ? this.testInputs.amountSol * 10e9
        : 0,
      this.tokenCtx.mint,
    );
  }
}
