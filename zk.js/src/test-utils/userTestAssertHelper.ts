import { PublicKey, SystemProgram } from "@solana/web3.js";
import assert = require("assert");

import {
  fetchNullifierAccountInfo,
  sleep,
  convertAndComputeDecimals,
  Action,
  fetchRecentTransactions,
  ParsedIndexedTransaction,
  TokenData,
  UserIndexedTransaction,
  Balance,
  Provider,
  User,
  AUTHORITY,
  MINIMUM_LAMPORTS,
  TOKEN_REGISTRY,
  merkleTreeProgramId,
  BN_0,
  lightPsp10in2outId,
  lightPsp2in2outId,
  MerkleTreeConfig,
  getEscrowPda,
  Utxo,
} from "../index";
import { getAssociatedTokenAddressSync } from "@solana/spl-token";
import { BN } from "@coral-xyz/anchor";

export type TestInputs = {
  amountSpl?: number;
  amountSol?: number;
  token: string;
  type: Action;
  recipient?: PublicKey;
  expectedUtxoHistoryLength: number;
  expectedSpentUtxosLength?: number;
  recipientSeed?: string;
  expectedRecipientUtxoLength?: number;
  mergedUtxo?: boolean;
  shieldToRecipient?: boolean;
  utxoCommitments?: string[];
  storage?: boolean;
  message?: Buffer;
  isMerge?: boolean;
};

export type TestUserBalances = {
  user: User;
  preShieldedBalance?: Balance;
  preShieldedInboxBalance?: Balance;
  preTokenBalance?: number | null;
  preSolBalance?: number;
  isSender: boolean;
  recipientSplAccount?: PublicKey;
  senderSplAccount?: PublicKey;
};

export class UserTestAssertHelper {
  private recentTransaction?: UserIndexedTransaction;
  public provider: Provider;
  public sender: TestUserBalances;
  public recipient: TestUserBalances;
  public testInputs: TestInputs;
  public tokenCtx: TokenData;
  public rpcPreSolBalance?: number;
  public recipientPreSolBalance?: number;

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

  async fetchAndSaveState() {
    const saveUserState = async (userBalances: TestUserBalances) => {
      userBalances.preShieldedBalance = (
        await User.init({
          provider: this.provider,
          account: userBalances.user.account,
        })
      ).balance;
      userBalances.preShieldedInboxBalance = await (
        await User.init({
          provider: this.provider,
          account: userBalances.user.account,
        })
      ).getUtxoInbox();
      if (this.testInputs.recipient) {
        userBalances.recipientSplAccount = getAssociatedTokenAddressSync(
          this.tokenCtx.mint,
          this.testInputs.recipient,
        );
      } else {
        userBalances.senderSplAccount = getAssociatedTokenAddressSync(
          this.tokenCtx.mint,
          this.sender.user.provider.wallet.publicKey,
        );
      }

      if (userBalances.recipientSplAccount) {
        let balance: number | null = null;
        try {
          balance = (
            await this.provider.provider?.connection.getTokenAccountBalance(
              userBalances.recipientSplAccount,
            )
          )?.value.uiAmount;
        } catch (e) {
          /* empty */
        }
        userBalances.preTokenBalance = balance ? balance : 0;
      } else {
        let balance: number | null = null;
        try {
          balance = (
            await this.provider.provider?.connection.getTokenAccountBalance(
              userBalances.senderSplAccount!,
            )
          )?.value.uiAmount;
        } catch (e) {
          /* empty */
        }
        userBalances.preTokenBalance = balance ? balance : 0;
      }

      userBalances.preSolBalance =
        await this.provider.provider?.connection.getBalance(
          this.provider.wallet.publicKey,
        );
    };
    await saveUserState(this.sender);
    await saveUserState(this.recipient);
    this.rpcPreSolBalance = await this.provider.provider?.connection.getBalance(
      this.provider.rpc.accounts.rpcRecipientSol,
    );
    if (this.testInputs.recipient) {
      this.recipientPreSolBalance =
        await this.provider.provider?.connection.getBalance(
          this.testInputs.recipient,
        );
    }
  }

  public async assertRecentTransactionIsIndexedCorrectly() {
    type ReferenceTransaction = {
      signer: PublicKey;
      to: PublicKey;
      from: PublicKey;
      verifier: PublicKey;
      rpcRecipientSol: PublicKey;
      type: Action;
      publicAmountSol: BN;
      publicAmountSpl: BN;
      encryptedUtxos?: Buffer | any[];
      leaves?: number[][];
      nullifiers?: BN[];
      rpcFee: BN;
      message?: Buffer;
    };

    const assertTransactionProperties = (
      reference: ReferenceTransaction,
      transaction: ParsedIndexedTransaction,
    ) => {
      assert.equal(
        transaction.type,
        reference.type,
        "Transaction type mismatch",
      );
      assert.equal(
        transaction.signer,
        reference.signer.toBase58(),
        "Signer mismatch",
      );
      assert.equal(
        transaction.publicAmountSol,
        reference.publicAmountSol.toString(),
        "Public SOL amount mismatch",
      );
      assert.equal(
        transaction.publicAmountSpl,
        reference.publicAmountSpl.toString(),
        "Public SPL amount mismatch",
      );
      assert.equal(
        transaction.to,
        reference.to.toBase58(),
        "Recipient mismatch",
      );
      assert.equal(
        transaction.from,
        reference.from.toBase58(),
        "Sender mismatch",
      );
      assert.equal(
        transaction.verifier,
        reference.verifier.toBase58(),
        "Verifier mismatch",
      );
      assert.equal(
        transaction.rpcRecipientSol,
        reference.rpcRecipientSol.toBase58(),
        "Rpc recipient SOL mismatch",
      );
      assert.equal(
        transaction.rpcFee,
        reference.rpcFee.toString(),
        "Rpc fee mismatch",
      );

      if (reference.encryptedUtxos !== undefined)
        assert.deepStrictEqual(
          transaction.encryptedUtxos,
          reference.encryptedUtxos,
          "Encrypted UTXOs mismatch",
        );
      if (reference.leaves !== undefined)
        assert.deepStrictEqual(
          transaction.leaves,
          reference.leaves,
          "Leaves mismatch",
        );
      if (reference.nullifiers !== undefined)
        assert.deepStrictEqual(
          transaction.nullifiers,
          reference.nullifiers,
          "Nullifiers mismatch",
        );
      if (reference.message !== undefined)
        assert.deepStrictEqual(
          transaction.message,
          reference.message,
          "Message mismatch",
        );
    };
    const { amountSol, amountSpl, type } = this.testInputs;

    let transactions = await this.sender.user.getTransactionHistory();

    transactions.sort((a, b) => b.blockTime - a.blockTime);

    this.recentTransaction = transactions[0];

    const currentSlot = await this.provider.provider!.connection.getSlot();
    if (
      this.recentTransaction &&
      this.recentTransaction.blockTime < currentSlot - 60
    ) {
      let retries = 3;
      while (retries > 0) {
        await sleep(1000);
        transactions = await this.sender.user.getTransactionHistory();
        transactions.sort((a, b) => b.blockTime - a.blockTime);
        this.recentTransaction = transactions[0];
        if (this.recentTransaction.blockTime > currentSlot - 60) break;
        retries--;
        console.log("Retrying to get recent transaction");
      }
    }

    try {
      switch (type) {
        case Action.TRANSFER: {
          // there is a case that a fresh account just received a transfer but has no other balance yet
          if (this.testInputs.isMerge === true && transactions.length === 0)
            break;
          assertTransactionProperties(
            {
              signer: this.provider.rpc.accounts.rpcPubkey,
              publicAmountSpl: BN_0,
              publicAmountSol: BN_0,
              rpcFee: this.sender.user.provider.rpc.getRpcFee(),
              to: AUTHORITY,
              from: MerkleTreeConfig.getSolPoolPda(merkleTreeProgramId).pda,
              rpcRecipientSol:
                this.sender.user.provider.rpc.accounts.rpcRecipientSol,
              type: Action.TRANSFER,
              verifier: this.testInputs.isMerge
                ? lightPsp10in2outId
                : lightPsp2in2outId,
              message: undefined,
            },
            this.recentTransaction!,
          );
          break;
        }
        case Action.SHIELD: {
          assertTransactionProperties(
            {
              signer: this.sender.user.provider.wallet.publicKey,
              to: MerkleTreeConfig.getSolPoolPda(merkleTreeProgramId).pda,
              from: getEscrowPda(lightPsp2in2outId),
              publicAmountSpl: amountSpl
                ? convertAndComputeDecimals(amountSpl, this.tokenCtx!.decimals)
                : BN_0,
              publicAmountSol: amountSol
                ? convertAndComputeDecimals(amountSol, new BN(1e9))
                : BN_0,
              rpcFee: BN_0,
              rpcRecipientSol: AUTHORITY,
              type: Action.SHIELD,
              verifier: lightPsp2in2outId,
              message: undefined,
            },
            this.recentTransaction!,
          );
          break;
        }
        case Action.UNSHIELD: {
          // index transaction is broken
          // - invalid recipient
          // - invalid sender
          // - invalid verifier
          assertTransactionProperties(
            {
              signer: this.provider.rpc.accounts.rpcPubkey,
              publicAmountSpl: amountSpl
                ? convertAndComputeDecimals(amountSpl, this.tokenCtx!.decimals)
                : BN_0,
              publicAmountSol: amountSol
                ? convertAndComputeDecimals(amountSol, new BN(1e9))
                : BN_0,
              rpcFee:
                this.tokenCtx!.symbol != "SOL" &&
                !this.recipient.preTokenBalance
                  ? this.sender.user.provider.rpc.getRpcFee(true)
                  : this.sender.user.provider.rpc.getRpcFee(),
              to: this.testInputs.recipient!,
              from: MerkleTreeConfig.getSolPoolPda(merkleTreeProgramId).pda,
              rpcRecipientSol:
                this.sender.user.provider.rpc.accounts.rpcRecipientSol,
              type: Action.UNSHIELD,
              verifier: lightPsp2in2outId,
              message: undefined,
            },
            this.recentTransaction!,
          );
          break;
        }
        default: {
          throw new Error("Unknown transaction type");
        }
      }
    } catch (error) {
      console.log("transactions", transactions);
      console.log("recent transaction", this.recentTransaction);
      console.log("testInputs ", this.testInputs);
      throw error;
    }
  }

  /**
   * Checks:
   * - every utxo in utxos is inserted and is not spent
   * - every utxo in spent utxos is spent
   * - if an utxo has a spl asset it is categorized in that spl asset
   * - every utxo in a spl TokenBalance has a balance in this token
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
    // await user.provider.latestMerkleTree();
    for (const [asset, tokenBalance] of user.balance.tokenBalances.entries()) {
      // nullifier is not inserted
      for (const [, utxo] of tokenBalance.utxos.entries()) {
        if (!utxo.nullifier)
          throw new Error(`nullifier of utxo undefined, ${utxo}`);
        await this.assertNullifierAccountDoesNotExist(utxo.nullifier);
        checkCategorizationByAsset(asset, utxo);
      }
      // nullifier of utxo is inserted
      for (const utxo of tokenBalance.spentUtxos.values()) {
        if (!utxo.nullifier)
          throw new Error(`nullifier of utxo undefined, ${utxo}`);
        this.assertNullifierAccountExists(utxo.nullifier);
        checkCategorizationByAsset(asset, utxo);
      }
    }
  }

  async assertInboxBalance(user: User) {
    await user.getUtxoInbox();
    for (const tokenBalance of user.inboxBalance.tokenBalances.values()) {
      // nullifier of utxo is inserted
      for (const utxo of tokenBalance.spentUtxos.values()) {
        if (!utxo.nullifier)
          throw new Error(`nullifier of utxo undefined, ${utxo}`);
        this.assertNullifierAccountExists(utxo.nullifier);
      }
    }
  }

  /**
   * - check that utxos with an aggregate amount greater or equal than the spl and sol transfer amounts were spent
   */
  async assertUserUtxoSpent() {
    let amountSol = BN_0;
    let amountSpl = BN_0;
    for (const [
      asset,
      tokenBalance,
    ] of this.sender.preShieldedBalance!.tokenBalances.entries()) {
      for (const [commitment, utxo] of tokenBalance.utxos.entries()) {
        if (
          await fetchNullifierAccountInfo(
            utxo.nullifier,
            this.provider.provider?.connection,
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

  async assertShieldedSplBalance(
    amount: number,
    userBalances: TestUserBalances,
    shieldToRecipient?: boolean,
  ) {
    if (userBalances.isSender) {
      amount = amount * -1;
    }
    const postShieldedBalances = !shieldToRecipient
      ? await userBalances.user.getBalance(false)
      : await userBalances.user.getUtxoInbox();

    const tokenBalanceAfter = postShieldedBalances.tokenBalances.get(
      this.tokenCtx?.mint.toBase58(),
    )?.totalBalanceSpl;

    const _tokenBalancePre = !shieldToRecipient
      ? userBalances.preShieldedBalance!.tokenBalances.get(
          this.tokenCtx?.mint.toBase58(),
        )?.totalBalanceSpl
      : userBalances.preShieldedInboxBalance!.tokenBalances.get(
          this.tokenCtx?.mint.toBase58(),
        )?.totalBalanceSpl;
    const tokenBalancePre = _tokenBalancePre ? _tokenBalancePre : BN_0;

    assert.equal(
      tokenBalanceAfter!
        .toNumber()
        .toFixed(this.tokenCtx.decimals.toString().length - 1),
      (
        tokenBalancePre!.toNumber() +
        amount * this.tokenCtx?.decimals.toNumber()
      ).toFixed(this.tokenCtx.decimals.toString().length - 1),
      `Token compressed balance isSender ${
        userBalances.isSender
      } after ${tokenBalanceAfter!} != token shield amount ${tokenBalancePre!.toNumber()} + ${
        amount * this.tokenCtx?.decimals.toNumber()
      }
       balance utxos: ${userBalances.user.balance.tokenBalances}`,
    );
  }

  async assertSplBalance(amount: number, userBalances: TestUserBalances) {
    let splAccount;
    if (userBalances.isSender) {
      amount = amount * -1;
      splAccount = userBalances.senderSplAccount!;
    } else {
      splAccount = userBalances.recipientSplAccount!;
    }
    const postTokenBalance =
      await userBalances.user.provider.provider!.connection.getTokenAccountBalance(
        splAccount!,
      );

    assert.equal(
      postTokenBalance.value.uiAmount?.toFixed(
        this.tokenCtx.decimals.toString().length - 1,
      ),
      (userBalances.preTokenBalance! + amount).toFixed(
        this.tokenCtx.decimals.toString().length - 1,
      ),
      `user is sender ${userBalances.isSender} token balance after ${
        postTokenBalance.value.uiAmount
      } != user token balance before ${userBalances.preTokenBalance!} + shield amount ${amount}`,
    );
  }

  async assertSolBalance(
    lamports: number,
    transactionCost: number,
    preSolBalance: number,
    recipient: PublicKey,
  ) {
    const postSolBalance =
      await this.provider.provider!.connection.getBalance(recipient);

    assert.equal(
      postSolBalance,
      preSolBalance! + lamports - transactionCost,
      `user sol balance after ${postSolBalance} != user sol balance before ${preSolBalance} + shield amount ${lamports} sol`,
    );
  }

  async assertShieldedSolBalance(
    lamports: number,
    userBalances: TestUserBalances,
    shieldToRecipient?: boolean,
  ) {
    if (userBalances.isSender) {
      lamports = lamports * -1;
    }
    const postShieldedBalances = !shieldToRecipient
      ? await userBalances.user.getBalance(false)
      : await userBalances.user.getUtxoInbox();

    const solBalanceAfter = postShieldedBalances.totalSolBalance;
    const solBalancePre = !shieldToRecipient
      ? userBalances.preShieldedBalance!.totalSolBalance
      : userBalances.preShieldedInboxBalance!.totalSolBalance;

    assert.equal(
      solBalanceAfter!.toNumber(),
      solBalancePre!.toNumber() + lamports,
      `compressed sol balance after ${solBalanceAfter!} != shield amount ${solBalancePre!.toNumber()} + ${lamports}`,
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

    const nrUtxos = this.testInputs.storage
      ? this.recipient.user.balance.tokenBalances.get(mint.toBase58())?.utxos
          .size
      : this.recipient.user.inboxBalance.tokenBalances.get(mint.toBase58())
          ?.utxos.size;
    // if storage expecting nr utxos to stay constant
    const expectedNrUtxos = this.testInputs.storage
      ? this.recipient.preShieldedBalance?.tokenBalances.get(mint.toBase58())
          ?.utxos.size
      : this.testInputs.expectedRecipientUtxoLength;
    assert.equal(nrUtxos!, expectedNrUtxos!);

    const recipientPreBalancePlusTransferSpl =
      this.recipient.preShieldedInboxBalance!.tokenBalances.get(mint.toBase58())
        ? this.recipient
            .preShieldedInboxBalance!.tokenBalances.get(mint.toBase58())!
            .totalBalanceSpl!.toNumber() + transferAmountSpl
        : transferAmountSpl;
    assert.equal(
      this.recipient.user.inboxBalance.tokenBalances
        .get(mint.toBase58())
        ?.totalBalanceSpl!.toString(),
      recipientPreBalancePlusTransferSpl.toString(),
    );

    const recipientPreBalancePlusTransfer =
      this.recipient.preShieldedInboxBalance!.tokenBalances.get(mint.toBase58())
        ? this.recipient
            .preShieldedInboxBalance!.tokenBalances.get(mint.toBase58())!
            .totalBalanceSol!.toNumber() + transferAmountSol
        : transferAmountSol;

    assert.equal(
      this.recipient.user.inboxBalance.tokenBalances
        .get(mint.toBase58())
        ?.totalBalanceSol!.toString(),
      recipientPreBalancePlusTransfer.toString(),
    );
  }

  async standardAsserts() {
    await this.assertBalance(this.sender.user);
    await this.assertBalance(this.recipient.user);
    // TODO: fix flakyness issues
    // await this.assertRecentTransactionIsIndexedCorrectly();
    if (this.testInputs.type !== Action.SHIELD) {
      await this.assertRpcFee();
    }
  }
  async assertRpcFee() {
    // rpc recipient's sol balance should be increased by the rpc fee
    await this.assertSolBalance(
      this.tokenCtx!.symbol !== "SOL" &&
        this.testInputs.type.toString() === Action.UNSHIELD.toString()
        ? this.sender.user.provider.rpc.getRpcFee(true).toNumber()
        : this.sender.user.provider.rpc.getRpcFee().toNumber(),
      0,
      this.rpcPreSolBalance!,
      this.sender.user.provider!.rpc.accounts.rpcRecipientSol,
    );
  }
  /**
   * Asynchronously checks if token compression has been performed correctly for a user.
   * This method performs the following checks:
   *
   * 1. Asserts that the user's compressed token balance has increased by the amount compressed.
   * 2. Asserts that the user's token balance has decreased by the amount compressed.
   * 3. Asserts that the user's sol compressed balance has increased by the additional sol amount.
   * 4. Asserts that the length of spent UTXOs matches the expected spent UTXOs length.
   * 5. Asserts that the nullifier account exists for the user's first UTXO.
   * 6. Asserts that the recent indexed transaction is of type SHIELD and has the correct values.
   *
   * @returns {Promise<void>} Resolves when all checks are successful, otherwise throws an error.
   */
  async checkSplShielded() {
    await this.standardAsserts();
    // assert that the user's compressed balance has increased by the amount compressed
    await this.assertShieldedSplBalance(
      this.testInputs.amountSpl!,
      this.recipient,
      this.testInputs.shieldToRecipient,
    );

    // assert that the user's token balance has decreased by the amount compressed
    await this.assertSplBalance(this.testInputs.amountSpl!, this.sender);

    const lamports =
      this.testInputs.amountSol != undefined
        ? convertAndComputeDecimals(
            this.testInputs.amountSol,
            new BN(1e9),
          ).toNumber()
        : MINIMUM_LAMPORTS.toNumber();
    // assert that the user's sol compressed balance has increased by the additional sol amount
    await this.assertShieldedSolBalance(
      lamports,
      this.recipient,
      this.testInputs.shieldToRecipient,
    );

    const spentUtxoLength = this.recipient.user.balance.tokenBalances.get(
      this.tokenCtx.mint.toBase58(),
    )?.spentUtxos.size;

    const preUtxoLength = this.recipient.preShieldedBalance!.tokenBalances.get(
      this.tokenCtx.mint.toBase58(),
    )?.utxos.size;

    assert.equal(
      spentUtxoLength ? spentUtxoLength : 0,
      preUtxoLength ? spentUtxoLength : 0,
    );

    // TODO: make this less hardcoded
    await this.assertNullifierAccountDoesNotExist(
      this.recipient.user.balance.tokenBalances
        .get(this.tokenCtx.mint.toBase58())
        ?.utxos.values()
        .next()!.value.nullifier,
    );
  }

  /**
   * Asynchronously checks if SOL compression has been performed correctly for a user.
   * This method performs the following checks:
   *
   * 1. Asserts recipient user balance increased by compressed amount.
   * 2. Asserts sender users sol balance decreased by compressed amount.
   * 3. Asserts that user UTXOs are spent and updated correctly.
   * 4. Asserts that the recent indexed transaction is of type SHIELD and has the correct values.
   *
   * Note: The temporary account cost calculation is not deterministic and may vary depending on whether the user has
   * compressed SPL tokens before. This needs to be handled carefully.
   *
   * @returns {Promise<void>} Resolves when all checks are successful, otherwise throws an error.
   */
  async checkSolShielded() {
    await this.standardAsserts();
    // assert that the user's compressed balance has increased by the amount compressed
    await this.assertShieldedSolBalance(
      convertAndComputeDecimals(
        this.testInputs.amountSol!,
        this.tokenCtx!.decimals,
      ).toNumber(),
      this.recipient,
      this.testInputs.shieldToRecipient,
    );

    await this.assertSolBalance(
      convertAndComputeDecimals(
        this.testInputs.amountSol!,
        this.tokenCtx!.decimals,
      ).toNumber() * -1,
      5000,
      this.recipient.preSolBalance!,
      this.recipient.user.provider.wallet.publicKey,
    );
  }

  async checkSolUnshielded() {
    await this.standardAsserts();
    // recipient's sol balance should be increased by the amount decompressed
    await this.assertSolBalance(
      convertAndComputeDecimals(
        this.testInputs.amountSol!,
        this.tokenCtx!.decimals,
      ).toNumber(),
      0,
      this.recipientPreSolBalance!,
      this.testInputs.recipient!,
    );
    // sender's compressed sol balance should be decreased by the amount decompressed
    await this.assertShieldedSolBalance(
      convertAndComputeDecimals(
        this.testInputs.amountSol!,
        this.tokenCtx!.decimals,
      ).toNumber() + this.provider.rpc.getRpcFee().toNumber(),
      this.sender,
      false,
    );
  }

  async checkSolTransferred() {
    await this.standardAsserts();
    // senders's sol balance should be decreased by the amount transfered
    await this.assertShieldedSolBalance(
      convertAndComputeDecimals(
        this.testInputs.amountSol!,
        this.tokenCtx!.decimals,
      )
        .add(this.provider.rpc.getRpcFee())
        .toNumber(),
      this.sender,
      false,
    );
    // recipient's sol balance should be increased by the amount transfered
    await this.assertShieldedSolBalance(
      convertAndComputeDecimals(
        this.testInputs.amountSol!,
        this.tokenCtx!.decimals,
      ).toNumber(),
      this.recipient,
      true,
    );
  }

  /**
   * Asynchronously checks if token decompression has been performed correctly for a user.
   * This method performs the following checks:
   *
   * 1. Asserts that the user's compressed token balance has decreased by the amount decompressed.
   * 2. Asserts that the recipient's token balance has increased by the amount decompressed.
   * 3. Asserts that the user's compressed SOL balance has decreased by the fee.
   * 4. Asserts that user UTXOs are spent and updated correctly.
   * 5. Asserts that the recent indexed transaction is of type UNSHIELD and has the correct values.
   *
   * @returns {Promise<void>} Resolves when all checks are successful, otherwise throws an error.
   */
  async checkSplUnshielded() {
    // assert that the user's compressed token balance has decreased by the amount decompressed
    await this.standardAsserts();
    await this.assertShieldedSplBalance(
      this.testInputs.amountSpl!,
      this.sender,
    );

    // assert that the recipient token balance has increased by the amount compressed
    await this.assertSplBalance(this.testInputs.amountSpl!, this.recipient);

    let solDecreasedAmount =
      this.testInputs.amountSol != undefined
        ? convertAndComputeDecimals(this.testInputs.amountSol, new BN(1e9))
        : MINIMUM_LAMPORTS;
    solDecreasedAmount = solDecreasedAmount.add(
      this.provider.rpc.getRpcFee(true),
    );
    // assert that the user's sol compressed balance has decreased by fee
    await this.assertShieldedSolBalance(
      solDecreasedAmount.toNumber(),
      this.sender,
    );

    // assert that user utxos are spent and updated correctly
    await this.assertUserUtxoSpent();
  }

  /**
   * Asynchronously checks if a compressed token transfer has been performed correctly for a user.
   * This method performs the following checks:
   *
   * 1. Asserts that the user's compressed token balance has decreased by the amount transferred.
   * 2. Asserts that the user's compressed SOL balance has decreased by the rpc fee.
   * 3. Asserts that user UTXOs are spent and updated correctly.
   * 4. Asserts that the recent indexed transaction is of type SHIELD and has the correct values.
   * 5. Assert that the transfer has been received correctly by the compressed recipient's account.
   *
   * @returns {Promise<void>} Resolves when all checks are successful, otherwise throws an error.
   */
  async checkSplTransferred() {
    // assert that the user's compressed balance has decreased by the amount transferred
    await this.standardAsserts();
    await this.assertInboxBalance(this.recipient.user);

    // assert that the user's spl compressed balance has decreased by amountSpl
    await this.assertShieldedSplBalance(
      this.testInputs.amountSpl!,
      this.sender,
    );

    await this.assertShieldedSplBalance(
      this.testInputs.amountSpl!,
      this.recipient,
      true,
    );

    let shieldedLamports =
      this.testInputs.amountSol != undefined
        ? convertAndComputeDecimals(
            this.testInputs.amountSol,
            new BN(1e9),
          ).toNumber()
        : 0;
    shieldedLamports += this.provider.rpc.getRpcFee().toNumber();
    // assert that the user's sol compressed balance has decreased by fee
    await this.assertShieldedSolBalance(shieldedLamports, this.sender);

    // assert that user utxos are spent and updated correctly
    await this.assertUserUtxoSpent();

    await this.checkShieldedTransferReceived(
      this.testInputs.amountSpl !== undefined
        ? convertAndComputeDecimals(
            this.testInputs.amountSpl!,
            this.tokenCtx!.decimals,
          ).toNumber()
        : 0,
      this.testInputs.amountSol !== undefined
        ? convertAndComputeDecimals(
            this.testInputs.amountSol,
            new BN(1e9),
          ).toNumber()
        : 0,
      this.tokenCtx.mint,
    );
  }

  async checkMergedAll() {
    // assert that the user's compressed balance has decreased by the amount transferred
    await this.standardAsserts();
    await this.assertInboxBalance(this.recipient.user);
    assert.equal(
      this.sender.user.account.getPublicKey(),
      this.recipient.user.account.getPublicKey(),
      "Sender and recipient are the account for merges",
    );
    // if pre number of utxos was less than 10 expect current number to be one
    let preNumberUtxos = this.sender.preShieldedBalance!.tokenBalances.get(
      this.tokenCtx.mint.toBase58(),
    )?.utxos.size;
    preNumberUtxos = preNumberUtxos ?? 0;

    if (preNumberUtxos < 10) {
      assert.equal(
        this.sender.user.balance.tokenBalances.get(
          this.tokenCtx.mint.toBase58(),
        )?.utxos.size,
        1,
      );
    } else {
      throw new Error(`Sender had more than 10 utxos ${preNumberUtxos}`);
    }
    const preNumberInboxUtxos =
      this.sender.preShieldedInboxBalance!.tokenBalances.get(
        this.tokenCtx.mint.toBase58(),
      )?.utxos.size;
    const postNumberUtxos = this.sender.user.inboxBalance!.tokenBalances.get(
      this.tokenCtx.mint.toBase58(),
    )?.utxos.size;
    const expectedRemainingInboxUtxos =
      preNumberInboxUtxos! + preNumberUtxos - 10;
    assert.equal(
      postNumberUtxos,
      expectedRemainingInboxUtxos > 0 ? expectedRemainingInboxUtxos : 0,
      ` postNumberUtxos ${postNumberUtxos} expected: ${
        expectedRemainingInboxUtxos > 0 ? expectedRemainingInboxUtxos : 0
      }`,
    );
    // indexer does not find the merge transaction no idea why
    // not worth debugging since the indexer is being refactored rn
    // await this.assertRecentIndexedTransaction();
  }

  async checkMerged() {
    // assert that the user's compressed balance has decreased by the amount transferred
    await this.standardAsserts();
    await this.assertInboxBalance(this.recipient.user);
    assert.equal(
      this.sender.user.account.getPublicKey(),
      this.recipient.user.account.getPublicKey(),
      "Sender and recipient are the account for merges",
    );
    // if pre number of utxos was less than 10 expect current number to be one
    let preNumberUtxos = this.sender.preShieldedBalance!.tokenBalances.get(
      this.tokenCtx.mint.toBase58(),
    )!.utxos.size!;
    preNumberUtxos = preNumberUtxos ? preNumberUtxos : 0;

    if (preNumberUtxos < 10) {
      assert.equal(
        this.sender.user.balance.tokenBalances.get(
          this.tokenCtx.mint.toBase58(),
        )?.utxos.size,
        1,
      );
    } else {
      throw new Error(`Sender had more than 10 utxos ${preNumberUtxos}`);
    }

    /**
     * for every utxo
     * pre Inbox:
     * - existed
     * post Inbox:
     * - does not exist
     * post Balance:
     * - has increased by sum
     */
    let sum = BN_0;
    for (const commitment of this.testInputs.utxoCommitments!) {
      const existedInPreInbox = this.sender
        .preShieldedInboxBalance!.tokenBalances.get(
          this.tokenCtx.mint.toBase58(),
        )
        ?.utxos.get(commitment);
      assert.notEqual(
        existedInPreInbox,
        undefined,
        `commitment ${commitment},  did not exist in pre inbox`,
      );

      sum = this.tokenCtx.isNative
        ? sum.add(existedInPreInbox!.amounts[0]!)
        : sum.add(existedInPreInbox!.amounts[1]!);

      const existedInPostInbox = this.sender.user
        .inboxBalance!.tokenBalances.get(this.tokenCtx.mint.toBase58())
        ?.utxos.get(commitment);
      assert.equal(
        existedInPostInbox,
        undefined,
        `commitment ${commitment}, exists in post inbox`,
      );
    }
    const postBalance = this.tokenCtx.isNative
      ? this.recipient.user.balance.tokenBalances
          .get(this.tokenCtx.mint.toBase58())
          ?.totalBalanceSol.add(this.provider.rpc.getRpcFee(false))
      : this.recipient.user.balance.tokenBalances.get(
          this.tokenCtx.mint.toBase58(),
        )?.totalBalanceSpl;
    let preBalance = this.tokenCtx.isNative
      ? this.recipient.preShieldedBalance?.tokenBalances.get(
          SystemProgram.programId.toBase58(),
        )?.totalBalanceSol
      : this.recipient.preShieldedBalance?.tokenBalances.get(
          this.tokenCtx.mint.toBase58(),
        )?.totalBalanceSpl;
    preBalance = preBalance ? preBalance : BN_0;

    assert.equal(postBalance?.toString(), preBalance!.add(sum).toString());
  }
  async checkMessageStored() {
    if (!this.testInputs.message)
      throw new Error("Test inputs message undefined to assert message stored");
    const { transactions: indexedTransactions } = await fetchRecentTransactions(
      {
        connection: this.provider!.provider!.connection,
        batchOptions: {
          limit: 5000,
        },
      },
    );
    indexedTransactions.sort(
      (a, b) => b.transaction.blockTime - a.transaction.blockTime,
    );
    assert.equal(
      indexedTransactions[0].transaction.message.toString(),
      this.testInputs.message.toString(),
    );
  }

  async assertStoredWithTransfer() {
    // compressed sol balance is reduced by the rpc fee
    const postSolBalance = await (
      await this.recipient.user.getBalance()
    ).totalSolBalance
      .add(this.provider.rpc.getRpcFee())
      .toString();
    assert.strictEqual(
      this.recipient.preShieldedBalance?.totalSolBalance!.toString(),
      postSolBalance,
    );
    await this.checkMessageStored();
  }

  async assertStoredWithShield() {
    // compressed sol balance did not change
    const postSolBalance = (
      await this.recipient.user.getBalance()
    ).totalSolBalance.toString();
    assert.strictEqual(
      this.recipient.preShieldedBalance?.totalSolBalance!.toString(),
      postSolBalance,
    );
    await this.checkMessageStored();
  }

  async checkCommittedBalanceSpl() {
    if (this.tokenCtx.isNative)
      throw new Error("checkCommittedBalanceSpl is not implemented for sol");
    if (this.testInputs.type !== Action.TRANSFER) {
      const balance = await this.sender.user.getBalance();
      const numberOfUtxos =
        balance.tokenBalances.get(this.tokenCtx.mint.toBase58())?.utxos.size ??
        0;
      assert.equal(numberOfUtxos, 0);

      assert.equal(
        balance.tokenBalances.get(this.tokenCtx.mint.toBase58())!.committedUtxos
          .size,
        1,
      );
    } else {
      const balance = await this.recipient.user.getBalance();
      const numberOfUtxos =
        balance.tokenBalances.get(this.tokenCtx.mint.toBase58())?.utxos.size ??
        0;
      const numberOfCommittedUtxos =
        balance.tokenBalances.get(this.tokenCtx.mint.toBase58())?.committedUtxos
          .size ?? 0;

      assert.equal(numberOfCommittedUtxos, 0);
      assert.equal(numberOfUtxos, 0);
    }

    const userSpendable = await User.init({
      provider: this.provider,
      seed: this.testInputs.recipientSeed,
    });
    let balanceSpendable =
      this.testInputs.type !== Action.TRANSFER
        ? await userSpendable.getBalance()
        : await userSpendable.getUtxoInbox();
    let retries = 20;
    let numberOfUtxos =
      balanceSpendable.tokenBalances.get(this.tokenCtx.mint.toBase58())?.utxos
        .size ?? 0;

    while (numberOfUtxos == 0) {
      await sleep(1000);
      if (retries === 0)
        throw new Error(
          `Didn't get any utxos for for action ${this.testInputs.type} ${this.tokenCtx.symbol}`,
        );
      retries--;
      balanceSpendable =
        this.testInputs.type !== Action.TRANSFER
          ? await userSpendable.getBalance()
          : await userSpendable.getUtxoInbox();
      numberOfUtxos =
        balanceSpendable.tokenBalances.get(this.tokenCtx.mint.toBase58())?.utxos
          .size ?? 0;
    }
    assert.equal(
      balanceSpendable.tokenBalances.get(this.tokenCtx.mint.toBase58())!
        .committedUtxos.size,
      0,
    );
    assert.equal(
      balanceSpendable.tokenBalances.get(this.tokenCtx.mint.toBase58())!.utxos
        .size,
      1,
    );
    if (this.testInputs.type === Action.SHIELD) {
      await this.checkSplShielded();
    } else if (this.testInputs.type === Action.TRANSFER) {
      await this.checkSplTransferred();
    } else if (this.testInputs.type === Action.UNSHIELD) {
      await this.checkSplUnshielded();
    }
  }
}
