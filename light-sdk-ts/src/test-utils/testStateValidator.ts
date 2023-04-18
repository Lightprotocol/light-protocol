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

type TestInputs = {
  amountSpl: number;
  amountSol: number;
  token: string;
  type: Action;
  recipientSpl?: PublicKey;
  utxos: number;
  spentUtxos?: number;
};

export class TestStateValidator {
  private preShieldedBalances?: Balance[];
  private preTokenBalance?: any;
  private preRecipientTokenBalance?: any;
  private preSolBalance?: number;
  private recentTransaction?: indexedTransaction;
  private preUtxos?: Utxo[];

  public provider: Provider;
  public user: User;
  public userSplAccount?: PublicKey;
  public recipientSplAccount?: PublicKey;
  public testInputs: TestInputs;
  public tokenCtx: any;

  constructor({
    user,
    provider,
    testInputs,
  }: {
    user: User;
    provider: Provider;
    testInputs: TestInputs;
  }) {
    this.user = user;
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

      this.preShieldedBalances = await this.user.getBalance({ latest: true });

      this.preTokenBalance = this.userSplAccount
        ? await this.provider.provider?.connection.getTokenAccountBalance(
            this.userSplAccount,
          )
        : undefined;

      this.preSolBalance = await this.provider.provider?.connection.getBalance(
        this.provider.wallet.publicKey,
      );

      this.provider = this.provider;

      this.preUtxos = this.user.utxos;
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
        this.user.utxos![0]._commitment,
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

  async assertUserUtxos() {
    let commitmentIndex = this.user.spentUtxos!.findIndex(
      (utxo) => utxo._commitment === this.user.utxos![0]._commitment,
    );

    let commitmentSpent = this.user.utxos!.findIndex(
      (utxo) => utxo._commitment === this.preUtxos![0]._commitment,
    );

    this.assertNullifierAccountExists(this.user.utxos![0]._nullifier!);

    assert.equal(this.user.utxos!.length, this.testInputs.utxos);
    assert.equal(commitmentIndex, -1);
    assert.equal(commitmentSpent, -1);
  }

  async assertShieldedTokenBalance(amount: number) {
    const postShieldedBalances = await this.user.getBalance({ latest: true });

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
    const postShieldedBalances = await this.user.getBalance({ latest: true });

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

  async checkTokenShielded() {
    // assert that the user's shielded balance has increased by the amount shielded
    await this.assertShieldedTokenBalance(this.testInputs.amountSpl);

    // assert that the user's token balance has decreased by the amount shielded
    const tokenDecreasedAmount = this.testInputs.amountSpl * -1;

    await this.assertTokenBalance(tokenDecreasedAmount);

    // assert that the user's sol shielded balance has increased by the additional sol amount
    await this.assertShieldedSolBalance(150000);

    assert.equal(this.user.spentUtxos!.length, this.testInputs.spentUtxos);

    await this.assertNullifierAccountExists(this.user.utxos![0]._nullifier!);

    // assert that recentIndexedTransaction is of type SHIELD and have right values
    await this.assertRecentIndexedTransaction();
  }

  async checkSolShielded() {
    // assert that the user's shielded balance has increased by the amount shielded
    await this.assertShieldedSolBalance(
      this.testInputs.amountSol * this.tokenCtx?.decimals.toNumber(),
    );

    const tempAccountCost = 3502840 - 1255000; //x-y nasty af. underterministic: costs more(y) if shielded SPL before!

    // assert that the user's sol balance has decreased by the amount
    const solDecreasedAmount = this.testInputs.amountSol * -1;

    await this.assertSolBalance(solDecreasedAmount, tempAccountCost);

    // assert that user utxos are spent and updated correctly
    await this.assertUserUtxos();

    // assert that recentIndexedTransaction is of type SHIELD and have right values
    await this.assertRecentIndexedTransaction();
  }

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
    await this.assertUserUtxos();

    // // assert that recentIndexedTransaction is of type UNSHIELD and have right values
    await this.assertRecentIndexedTransaction();
  }

  async checkTokenTransferred() {
    // assert that the user's shielded balance has decreased by the amount transferred

    const tokenDecreasedAmount = this.testInputs.amountSpl * -1;

    await this.assertShieldedTokenBalance(tokenDecreasedAmount);

    // assert that the user's sol shielded balance has decreased by fee
    const solDecreasedAmount = this.provider.relayer.relayerFee.toNumber() * -1;

    await this.assertShieldedSolBalance(solDecreasedAmount);

    // assert that user utxos are spent and updated correctly
    await this.assertUserUtxos();

    // assert that recentIndexedTransaction is of type SHIELD and have right values
    await this.assertRecentIndexedTransaction();
  }
}
