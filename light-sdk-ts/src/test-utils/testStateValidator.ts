import { PublicKey, SystemProgram } from "@solana/web3.js";
import { assert, use } from "chai";
import { fetchNullifierAccountInfo } from "../utils";
import { Utxo } from "../utxo";
import { Action } from "../transaction";
import { indexedTransaction } from "../types";
import { Balance, Provider, User } from "../wallet";

export class TestStateValidator {
  private preShieldedBalances?: Balance[];
  private preTokenBalance?: any;
  private preRecipientTokenBalance?: any;
  private preSolBalance?: number;
  private recentTransaction?: indexedTransaction;
  private indexedTransactionsLength = 1;
  private utxos?: Utxo[];
  private spentUtxosLength: number = 1;
  public provider?: Provider;

  async fetchAndSaveState({
    user,
    provider,
    userSplAccount,
    recipientSplAccount,
  }: {
    user: User;
    provider: Provider;
    userSplAccount?: PublicKey;
    recipientSplAccount?: PublicKey;
  }) {
    this.preShieldedBalances = await user.getBalance({ latest: true });
    this.preTokenBalance = userSplAccount
      ? await provider.provider?.connection.getTokenAccountBalance(
          userSplAccount,
        )
      : undefined;
    this.preRecipientTokenBalance = recipientSplAccount
      ? await provider.provider?.connection.getTokenAccountBalance(
          recipientSplAccount,
        )
      : undefined;
    this.preSolBalance = await provider.provider?.connection.getBalance(
      provider.wallet.publicKey,
    );
    this.provider = provider;
    this.utxos = user.utxos;
  }

  public async assertRecentIndexedTransaction({
    amountSol,
    amountSpl,
    tokenCtx,
    user,
    type,
    recipientSplAccount,
  }: {
    amountSol?: number;
    amountSpl?: number;
    tokenCtx?: any;
    user: User;
    type: Action;
    recipientSplAccount?: PublicKey;
  }) {
    const indexedTransactions =
      await this.provider!.relayer.getIndexedTransactions(
        this.provider!.provider!.connection,
      );

    this.recentTransaction = indexedTransactions[0];

    assert.strictEqual(
      indexedTransactions.length,
      this.indexedTransactionsLength,
    );

    if (amountSpl !== undefined) {
      assert.strictEqual(
        this.recentTransaction.amountSpl.div(tokenCtx!.decimals).toNumber(),
        amountSpl,
      );
    }

    if (amountSol !== undefined) {
      assert.strictEqual(
        this.recentTransaction.amountSol.div(tokenCtx!.decimals).toNumber(),
        amountSol,
      );
    }

    if (type === Action.SHIELD) {
      assert.strictEqual(
        this.recentTransaction.from.toBase58(),
        this.provider!.wallet.publicKey.toBase58(),
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

    if (recipientSplAccount !== undefined) {
      assert.strictEqual(
        this.recentTransaction.to.toBase58(),
        recipientSplAccount.toBase58(),
      );
    }

    if (type !== Action.TRANSFER) {
      assert.strictEqual(
        this.recentTransaction.commitment,
        user.utxos![0]._commitment,
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
        : this.provider!.relayer.accounts.relayerRecipientSol.toBase58(),
    );

    this.indexedTransactionsLength += 1;
  }

  async assertUserUtxos(user: User) {
    let commitmentIndex = user.spentUtxos!.findIndex(
      (utxo) => utxo._commitment === user.utxos![0]._commitment,
    );

    let commitmentSpent = user.utxos!.findIndex(
      (utxo) => utxo._commitment === this.utxos![0]._commitment,
    );

    assert.notEqual(
      fetchNullifierAccountInfo(
        user.utxos![0]._nullifier!,
        this.provider!.provider!.connection,
      ),
      null,
    );

    assert.equal(user.spentUtxos!.length, this.spentUtxosLength);
    assert.equal(user.utxos!.length, 1);
    assert.equal(commitmentIndex, -1);
    assert.equal(commitmentSpent, -1);
    this.spentUtxosLength = this.spentUtxosLength + 1;
  }

  async assertShieldedTokenBalance(user: User, tokenCtx: any, amount: number) {
    const postShieldedBalances = await user.getBalance({ latest: true });

    let tokenBalanceAfter = postShieldedBalances.find(
      (b) => b.tokenAccount.toBase58() === tokenCtx?.tokenAccount.toBase58(),
    );
    let tokenBalancePre = this.preShieldedBalances!.find(
      (b) => b.tokenAccount.toBase58() === tokenCtx?.tokenAccount.toBase58(),
    );

    assert.equal(
      tokenBalanceAfter!.amount.toNumber(),
      tokenBalancePre!.amount.toNumber() +
        amount * tokenCtx?.decimals.toNumber(),
      `Token shielded balance after ${
        tokenBalanceAfter!.amount
      } != token shield amount ${tokenBalancePre!.amount.toNumber()} + ${
        amount * tokenCtx?.decimals.toNumber()
      }`,
    );
  }

  async assertTokenBalance(userSplAccount: PublicKey, amount: number) {
    const postTokenBalance =
      await this.provider!.provider!.connection.getTokenAccountBalance(
        userSplAccount,
      );

    assert.equal(
      postTokenBalance.value.uiAmount,
      this.preTokenBalance.value.uiAmount + amount,
      `user token balance after ${postTokenBalance.value.uiAmount} != user token balance before ${this.preTokenBalance.value.uiAmount} + shield amount ${amount}`,
    );
  }

  async assertSolBalance(
    amount: number,
    tokenCtx: any,
    tempAccountCost: number,
  ) {
    const postSolBalance = await this.provider!.provider!.connection.getBalance(
      this.provider!.wallet.publicKey,
    );

    assert.equal(
      postSolBalance,
      this.preSolBalance! -
        amount * tokenCtx.decimals.toNumber() +
        tempAccountCost,
      `user token balance after ${postSolBalance} != user token balance before ${this.preSolBalance} + shield amount ${amount} sol`,
    );
  }

  async assertRecipientTokenBalance(
    recipientSplAccount: PublicKey,
    amount: number,
  ) {
    const postRecipientTokenBalance =
      await this.provider!.provider!.connection.getTokenAccountBalance(
        recipientSplAccount,
      );

    assert.equal(
      postRecipientTokenBalance.value.uiAmount,
      this.preRecipientTokenBalance.value.uiAmount + amount,
      `recipient token balance after ${postRecipientTokenBalance.value.uiAmount} != recipient token balance before ${this.preRecipientTokenBalance.value.uiAmount} + shield amount ${amount}`,
    );
  }

  async assertShieledSolBalance(user: User, amount: number) {
    const postShieldedBalances = await user.getBalance({ latest: true });

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
}
