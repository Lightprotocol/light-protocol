import { PublicKey, SystemProgram } from "@solana/web3.js";
import { assert } from "chai";
import { fetchNullifierAccountInfo } from "../utils";
import { Utxo } from "../utxo";
import { Action } from "../transaction";
import { indexedTransaction } from "../types";
import { Balance, Provider, User } from "../wallet";
import { getAssociatedTokenAddressSync } from "@solana/spl-token";
import {
  MINIMUM_LAMPORTS,
  TOKEN_ACCOUNT_FEE,
  TOKEN_REGISTRY,
} from "../constants";
import { Account } from "index";

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

export class TestStateValidator {
  private preShieldedBalances?: Balance[];
  private preTokenBalance?: any;
  private preRecipientTokenBalance?: any;
  private preSolBalance?: number;
  private recentTransaction?: indexedTransaction;
  private preUtxos?: Utxo[];

  public provider: Provider;
  public userSender: User;
  public userRecipient: User;
  public userSplAccount?: PublicKey;
  public recipientSplAccount?: PublicKey;
  public testInputs: TestInputs;
  public tokenCtx: any;

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
    this.userSender = userSender;
    this.userRecipient = userRecipient;
    this.provider = provider;
    this.testInputs = testInputs;
  }

  async fetchAndSaveState() {
    try {
      const tokenCtx = TOKEN_REGISTRY.find(
        (t) => t.symbol === this.testInputs.token,
      );

      this.tokenCtx = tokenCtx;

      this.userSplAccount =
        this.testInputs.token !== "SOL"
          ? getAssociatedTokenAddressSync(
              tokenCtx!.tokenAccount,
              this.provider.wallet.publicKey,
            )
          : undefined;

      if (this.testInputs.recipientSpl) {
        this.recipientSplAccount = getAssociatedTokenAddressSync(
          tokenCtx!.tokenAccount,
          this.testInputs.recipientSpl,
        );

        this.preRecipientTokenBalance =
          await this.provider.provider?.connection.getTokenAccountBalance(
            this.recipientSplAccount,
          );
      }

      this.preShieldedBalances = await this.userSender.getBalance({
        latest: true,
      });

      this.preTokenBalance = this.userSplAccount
        ? await this.provider.provider?.connection.getTokenAccountBalance(
            this.userSplAccount,
          )
        : undefined;

      this.preSolBalance = await this.provider.provider?.connection.getBalance(
        this.provider.wallet.publicKey,
      );

      this.provider = this.provider;

      this.preUtxos = this.userSender.utxos;
    } catch (error) {
      console.log("error while fetching the state", { error });
    }
  }

  public async assertRecentIndexedTransaction() {
    const { amountSol, amountSpl, type } = this.testInputs;

    const indexedTransactions =
      await this.provider.relayer.getIndexedTransactions(
        this.provider.provider!.connection,
      );

    this.recentTransaction = indexedTransactions[0];

    if (amountSpl && type) {
      assert.strictEqual(
        this.recentTransaction.amountSpl
          .div(this.tokenCtx!.decimals)
          .toNumber(),
        type === Action.TRANSFER ? 0 : amountSpl,
      );
    }

    if (amountSol) {
      assert.strictEqual(
        this.recentTransaction.amountSol
          .div(this.tokenCtx!.decimals)
          .toNumber(),
        type === Action.TRANSFER ? 0 : amountSol,
      );
    }

    if (type === Action.SHIELD) {
      assert.strictEqual(
        this.recentTransaction.from.toBase58(),
        this.provider.wallet.publicKey.toBase58(),
      );
    }

    if (type === Action.TRANSFER) {
      assert.strictEqual(
        this.recentTransaction.to.toBase58(),
        PublicKey.default.toBase58(),
      );

      assert.strictEqual(
        this.recentTransaction.from.toBase58(),
        PublicKey.default.toBase58(),
      );
    }

    if (this.recipientSplAccount) {
      assert.strictEqual(
        this.recentTransaction.to.toBase58(),
        this.recipientSplAccount.toBase58(),
      );
    }

    if (type !== Action.TRANSFER) {
      assert.strictEqual(
        this.recentTransaction.commitment,
        this.userRecipient.utxos![0]._commitment,
      );
    }

    assert.strictEqual(this.recentTransaction.type, type);

    assert.strictEqual(
      this.recentTransaction.relayerFee.toString(),
      type === Action.UNSHIELD
        ? "500000"
        : type === Action.TRANSFER
        ? "100000"
        : "0",
    );

    assert.strictEqual(
      this.recentTransaction.relayerRecipientSol.toBase58(),
      type === Action.SHIELD
        ? PublicKey.default.toBase58()
        : this.provider.relayer.accounts.relayerRecipientSol.toBase58(),
    );
  }

  async assertUserUtxoSpent() {
    let commitmentIndex = this.userSender.spentUtxos!.findIndex(
      (utxo) => utxo._commitment === this.userSender.utxos![0]._commitment,
    );

    let commitmentSpent = this.userSender.utxos!.findIndex(
      (utxo) => utxo._commitment === this.preUtxos![0]._commitment,
    );

    this.assertNullifierAccountExists(this.userSender.utxos![0]._nullifier!);

    assert.equal(
      this.userSender.utxos!.length,
      this.testInputs.expectedUtxoHistoryLength,
    );
    assert.equal(commitmentIndex, -1);
    assert.equal(commitmentSpent, -1);
  }

  async assertShieldedTokenBalance(amount: number) {
    const postShieldedBalances = await this.userRecipient.getBalance({
      latest: true,
    });

    let tokenBalanceAfter = postShieldedBalances.find(
      (b) =>
        b.tokenAccount.toBase58() === this.tokenCtx?.tokenAccount.toBase58(),
    );
    let tokenBalancePre = this.preShieldedBalances!.find(
      (b) =>
        b.tokenAccount.toBase58() === this.tokenCtx?.tokenAccount.toBase58(),
    );

    assert.equal(
      tokenBalanceAfter!.amount.toNumber(),
      tokenBalancePre!.amount.toNumber() +
        amount * this.tokenCtx?.decimals.toNumber(),
      `Token shielded balance after ${
        tokenBalanceAfter!.amount
      } != token shield amount ${tokenBalancePre!.amount.toNumber()} + ${
        amount * this.tokenCtx?.decimals.toNumber()
      }`,
    );
  }

  async assertTokenBalance(amount: number) {
    const postTokenBalance =
      await this.provider.provider!.connection.getTokenAccountBalance(
        this.userSplAccount!,
      );

    assert.equal(
      postTokenBalance.value.uiAmount,
      this.preTokenBalance.value.uiAmount + amount,
      `user token balance after ${postTokenBalance.value.uiAmount} != user token balance before ${this.preTokenBalance.value.uiAmount} + shield amount ${amount}`,
    );
  }

  async assertSolBalance(amount: number, tempAccountCost: number) {
    const postSolBalance = await this.provider.provider!.connection.getBalance(
      this.provider.wallet.publicKey,
    );

    assert.equal(
      postSolBalance,
      this.preSolBalance! +
        amount * this.tokenCtx.decimals.toNumber() +
        tempAccountCost,
      `user token balance after ${postSolBalance} != user token balance before ${this.preSolBalance} + shield amount ${amount} sol`,
    );
  }

  async assertRecipientTokenBalance(amount: number) {
    const postRecipientTokenBalance =
      await this.provider.provider!.connection.getTokenAccountBalance(
        this.recipientSplAccount!,
      );

    assert.equal(
      postRecipientTokenBalance.value.uiAmount,
      this.preRecipientTokenBalance.value.uiAmount + amount,
      `recipient token balance after ${postRecipientTokenBalance.value.uiAmount} != recipient token balance before ${this.preRecipientTokenBalance.value.uiAmount} + shield amount ${amount}`,
    );
  }

  async assertShieldedSolBalance(amount: number) {
    const postShieldedBalances = await this.userRecipient.getBalance({
      latest: true,
    });

    let solBalanceAfter = postShieldedBalances.find(
      (b) => b.tokenAccount.toBase58() === SystemProgram.programId.toString(),
    );
    let solBalancePre = this.preShieldedBalances!.find(
      (b) => b.tokenAccount.toBase58() === SystemProgram.programId.toString(),
    );

    assert.equal(
      solBalanceAfter!.amount.toNumber(),
      solBalancePre!.amount.toNumber() + amount,
      `shielded sol balance after ${
        solBalanceAfter!.amount
      } != shield amount ${solBalancePre!.amount.toNumber()} + ${amount}`,
    );
  }

  async assertNullifierAccountExists(nullifier: string) {
    assert.notEqual(
      fetchNullifierAccountInfo(nullifier, this.provider.connection!),
      null,
    );
  }

  async checkShieldedTransferReceived() {
    // assert recipient utxo
    const userRecipient: User = await User.init(
      this.provider,
      this.testInputs.recipientSeed,
    );
    let { decryptedUtxos } = await userRecipient.getUtxos(false);
    assert.equal(
      decryptedUtxos.length,
      this.testInputs.expectedRecipientUtxoLength,
    );
    assert.equal(decryptedUtxos[0].amounts[1].toString(), "100");
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
    // assert that the user's shielded balance has increased by the amount shielded
    await this.assertShieldedTokenBalance(this.testInputs.amountSpl);

    // assert that the user's token balance has decreased by the amount shielded
    const tokenDecreasedAmount = this.testInputs.amountSpl * -1;

    await this.assertTokenBalance(tokenDecreasedAmount);

    // assert that the user's sol shielded balance has increased by the additional sol amount
    await this.assertShieldedSolBalance(150000);

    assert.equal(
      this.userSender.spentUtxos!.length,
      this.testInputs.expectedSpentUtxosLength,
    );

    await this.assertNullifierAccountExists(
      this.userSender.utxos![0]._nullifier!,
    );

    // assert that recentIndexedTransaction is of type SHIELD and have right values
    await this.assertRecentIndexedTransaction();
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
    // assert that the user's shielded balance has increased by the amount shielded
    await this.assertShieldedSolBalance(
      this.testInputs.amountSol * this.tokenCtx?.decimals.toNumber(),
    );

    const tempAccountCost = 3502840 - 1255000; //x-y nasty af. underterministic: costs more(y) if shielded SPL before!

    // assert that the user's sol balance has decreased by the amount
    const solDecreasedAmount = this.testInputs.amountSol * -1;

    await this.assertSolBalance(solDecreasedAmount, tempAccountCost);

    if (this.testInputs.mergedUtxo) {
      // assert that user utxos are spent and updated correctly
      await this.assertUserUtxoSpent();
    }

    // assert that recentIndexedTransaction is of type SHIELD and have right values
    await this.assertRecentIndexedTransaction();
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

    const tokenDecreasedAmount = this.testInputs.amountSpl * -1;

    await this.assertShieldedTokenBalance(tokenDecreasedAmount);

    // // assert that the recipient token balance has increased by the amount shielded
    await this.assertRecipientTokenBalance(this.testInputs.amountSpl);

    const solDecreasedAmount = (MINIMUM_LAMPORTS + TOKEN_ACCOUNT_FEE) * -1;

    // assert that the user's sol shielded balance has decreased by fee
    await this.assertShieldedSolBalance(solDecreasedAmount);

    // // assert that user utxos are spent and updated correctly
    await this.assertUserUtxoSpent();

    // // assert that recentIndexedTransaction is of type UNSHIELD and have right values
    await this.assertRecentIndexedTransaction();
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

    const tokenDecreasedAmount = this.testInputs.amountSpl * -1;

    await this.assertShieldedTokenBalance(tokenDecreasedAmount);

    // assert that the user's sol shielded balance has decreased by fee
    const solDecreasedAmount = this.provider.relayer.relayerFee.toNumber() * -1;

    await this.assertShieldedSolBalance(solDecreasedAmount);

    // assert that user utxos are spent and updated correctly
    await this.assertUserUtxoSpent();

    // assert that recentIndexedTransaction is of type SHIELD and have right values
    await this.assertRecentIndexedTransaction();

    await this.checkShieldedTransferReceived();
  }
}
