import {
  PublicKey,
  SystemProgram,
  Transaction as SolanaTransaction,
} from "@solana/web3.js";
import * as anchor from "@coral-xyz/anchor";
import * as splToken from "@solana/spl-token";
import { BN } from "@coral-xyz/anchor";
const circomlibjs = require("circomlibjs");
import {
  CreateUtxoErrorCode,
  UtxoErrorCode,
  ProviderErrorCode,
  RelayerErrorCode,
  TransactionErrorCode,
  TransactionParametersErrorCode,
  UserError,
  UserErrorCode,
  Provider,
  SolMerkleTree,
  SIGN_MESSAGE,
  AUTHORITY,
  SelectInUtxosErrorCode,
  TOKEN_REGISTRY,
  merkleTreeProgramId,
  Account,
  Utxo,
  convertAndComputeDecimals,
  Transaction,
  TransactionParameters,
  Action,
  getUpdatedSpentUtxos,
  AppUtxoConfig,
  createRecipientUtxos,
  Balance,
  InboxBalance,
  TokenUtxoBalance,
  NACL_ENCRYPTED_COMPRESSED_UTXO_BYTES_LENGTH,
  decryptAddUtxoToBalance,
  fetchNullifierAccountInfo,
  IndexedTransaction,
  getUserIndexTransactions,
  UserIndexedTransaction,
  IDL_VERIFIER_PROGRAM_ZERO,
  IDL_VERIFIER_PROGRAM_ONE,
  TRANSACTION_MERKLE_TREE_KEY,
  MAX_MESSAGE_SIZE,
  IDL_VERIFIER_PROGRAM_STORAGE,
  AccountErrorCode,
  ProgramUtxoBalance,
  TOKEN_PUBKEY_SYMBOL,
  MESSAGE_MERKLE_TREE_KEY,
  UtxoError,
  IDL_VERIFIER_PROGRAM_TWO,
  isProgramVerifier,
  TokenData,
  decimalConversion,
} from "../index";
import { Idl } from "@coral-xyz/anchor";
const message = new TextEncoder().encode(SIGN_MESSAGE);

// TODO: Utxos should be assigned to a merkle tree
// TODO: add support for wallet adapter (no access to payer keypair)

/**
 * This class represents a user in the system. It includes properties and methods that allow users to:
 * - Perform transactions.
 * - Manage balances.
 * - Interact with the provider.
 *
 */
export class User {
  provider: Provider;
  account: Account;
  transactionHistory?: UserIndexedTransaction[];
  private seed?: string;
  recentTransactionParameters?: TransactionParameters;
  recentTransaction?: Transaction;
  approved?: boolean;
  appUtxoConfig?: AppUtxoConfig;
  balance: Balance;
  inboxBalance: InboxBalance;
  verifierIdl: Idl;

  /**
   *
   * @remarks
   * - The User class is designed to work with a provider, which must be passed in as an argument during instantiation.
   * - It also takes an optional account parameter, which represents the user's account.
   * - The User class includes methods to transact with parameters, retrieve balance information, store data, and more.
   *
   * @param provider - An instance of a Provider, which can be either a nodeProvider or a browserProvider.
   * @param account - An optional parameter representing the user's account.
   * @param serializedUtxos - An optional Buffer object representing the user's unspent transaction outputs (UTXOs).
   * @param serialiezdSpentUtxos - An optional Buffer object representing the user's spent UTXOs.
   * @param transactionNonce - An optional parameter representing the current transaction nonce.
   * @param appUtxoConfig - An optional parameter for the app UTXO configuration.
   * @param verifierIdl - An optional parameter for the verifier interface description language (IDL). Defaults to IDL_VERIFIER_PROGRAM_ZERO if not provided.
   *
   * @throws `UserError`
   * - If no wallet is provided in the provider.
   * - If the provider is not properly initialized.
   * - If there is no app-enabled verifier defined when an appUtxoConfig is provided.
   */
  constructor({
    provider,
    serializedUtxos, // balance
    serialiezdSpentUtxos, // inboxBalance idk
    account,
    transactionNonce,
    appUtxoConfig,
    verifierIdl = IDL_VERIFIER_PROGRAM_ZERO,
  }: {
    provider: Provider;
    serializedUtxos?: Buffer;
    serialiezdSpentUtxos?: Buffer;
    account: Account;
    transactionNonce?: number;
    appUtxoConfig?: AppUtxoConfig;
    verifierIdl?: Idl;
  }) {
    if (!provider.wallet)
      throw new UserError(
        UserErrorCode.NO_WALLET_PROVIDED,
        "constructor",
        "No wallet provided",
      );

    if (!provider.lookUpTable || !provider.solMerkleTree || !provider.poseidon)
      throw new UserError(
        UserErrorCode.PROVIDER_NOT_INITIALIZED,
        "constructor",
        "Provider not properly initialized",
      );

    this.provider = provider;
    this.account = account;
    if (appUtxoConfig && !isProgramVerifier(verifierIdl))
      throw new UserError(
        UserErrorCode.VERIFIER_IS_NOT_APP_ENABLED,
        "constructor",
        `appUtxo config is provided but there is no app enabled verifier defined. The defined verifier is ${verifierIdl.name}.`,
      );
    this.appUtxoConfig = appUtxoConfig;
    this.verifierIdl = verifierIdl ? verifierIdl : IDL_VERIFIER_PROGRAM_ZERO;
    this.balance = {
      tokenBalances: new Map([
        [SystemProgram.programId.toBase58(), TokenUtxoBalance.initSol()],
      ]),
      programBalances: new Map(),
      nftBalances: new Map(),
      transactionNonce: 0,
      committedTransactionNonce: 0,
      decryptionTransactionNonce: 0,
      totalSolBalance: new BN(0),
    };
    this.inboxBalance = {
      tokenBalances: new Map([
        [SystemProgram.programId.toBase58(), TokenUtxoBalance.initSol()],
      ]),
      programBalances: new Map(),
      nftBalances: new Map(),
      transactionNonce: 0,
      committedTransactionNonce: 0,
      numberInboxUtxos: 0,
      decryptionTransactionNonce: 0,
      totalSolBalance: new BN(0),
    };
  }

  // TODO: should update merkle tree as well
  // TODO: test robustness
  // TODO: nonce incrementing is very ugly revisit

  /**
   * @async
   * This method updates the balance state by identifying spent UTXOs and decrypting new UTXOs.
   * - It iterates through the UTXOs in the balance and moves spent UTXOs to spentUtxos.
   * - It then fetches indexed transactions, decrypts the new UTXOs and adds them to the balance.
   * - Finally, it calculates the total Solana balance and updates the balance object.
   *
   * @param {boolean} aes - A flag to indicate whether AES encryption is used. Defaults to `true`.
   * @param {Balance | InboxBalance} balance - The balance to be updated. It can be either a `Balance` or an `InboxBalance`.
   * @param {PublicKey} merkleTreePdaPublicKey - The public key of the Merkle Tree PDA.
   *
   * @throws {UserError} UserError: When the provider is undefined or not initialized.
   *
   * @returns {Promise<Balance | InboxBalance>} A promise that resolves to the updated balance. It can be either a `Balance` or an `InboxBalance`.
   */
  async syncState(
    aes: boolean = true,
    balance: Balance | InboxBalance,
    merkleTreePdaPublicKey: PublicKey,
  ): Promise<Balance | InboxBalance> {
    // reduce balance by spent utxos
    if (!this.provider.provider)
      throw new UserError(
        UserErrorCode.PROVIDER_NOT_INITIALIZED,
        "syncState",
        "provider is undefined",
      );

    // identify spent utxos
    for (var [token, tokenBalance] of balance.tokenBalances) {
      for (var [key, utxo] of tokenBalance.utxos) {
        let nullifierAccountInfo = await fetchNullifierAccountInfo(
          utxo.getNullifier(this.provider.poseidon)!,
          this.provider.provider.connection,
        );
        if (nullifierAccountInfo !== null) {
          tokenBalance.movetToSpentUtxos(key);
        }
      }
    }

    if (!this.provider)
      throw new UserError(ProviderErrorCode.PROVIDER_UNDEFINED, "syncState");
    if (!this.provider.provider)
      throw new UserError(UserErrorCode.PROVIDER_NOT_INITIALIZED, "syncState");
    // TODO: adapt to indexedTransactions such that this works with verifier two for
    var decryptionTransactionNonce = balance.decryptionTransactionNonce;

    const indexedTransactions =
      await this.provider.relayer.getIndexedTransactions(
        this.provider.provider!.connection,
      );

    await this.provider.latestMerkleTree(indexedTransactions);

    for (const trx of indexedTransactions) {
      let leftLeafIndex = trx.firstLeafIndex.toNumber();

      for (let index = 0; index < trx.leaves.length; index += 2) {
        const leafLeft = trx.leaves[index];
        const leafRight = trx.leaves[index + 1];

        // transaction nonce is the same for all utxos in one transaction
        const tmpNonce = decryptionTransactionNonce;
        decryptionTransactionNonce = await decryptAddUtxoToBalance({
          encBytes: Buffer.from(
            trx.encryptedUtxos.slice(
              0,
              NACL_ENCRYPTED_COMPRESSED_UTXO_BYTES_LENGTH,
            ),
          ),
          index: leftLeafIndex,
          commitment: Buffer.from([...leafLeft]),
          account: this.account,
          poseidon: this.provider.poseidon,
          connection: this.provider.provider.connection,
          balance,
          merkleTreePdaPublicKey,
          leftLeaf: Uint8Array.from([...leafLeft]),
          aes,
          decryptionTransactionNonce: tmpNonce,
          verifierProgramLookupTable:
            this.provider.lookUpTables.verifierProgramLookupTable,
          assetLookupTable: this.provider.lookUpTables.assetLookupTable,
        });
        const decryptionTransactionNonce1 = await decryptAddUtxoToBalance({
          encBytes: Buffer.from(
            trx.encryptedUtxos.slice(
              128,
              128 + NACL_ENCRYPTED_COMPRESSED_UTXO_BYTES_LENGTH,
            ),
          ),
          index: leftLeafIndex + 1,
          commitment: Buffer.from([...leafRight]),
          account: this.account,
          poseidon: this.provider.poseidon,
          connection: this.provider.provider.connection,
          balance,
          merkleTreePdaPublicKey,
          leftLeaf: Uint8Array.from([...leafLeft]),
          aes,
          decryptionTransactionNonce: tmpNonce,
          verifierProgramLookupTable:
            this.provider.lookUpTables.verifierProgramLookupTable,
          assetLookupTable: this.provider.lookUpTables.assetLookupTable,
        });
        // handle case that only one utxo decrypted and assign incremented decryption transaction nonce accordingly
        decryptionTransactionNonce = decryptionTransactionNonce
          ? decryptionTransactionNonce
          : decryptionTransactionNonce1;
      }
    }

    balance.transactionNonce = decryptionTransactionNonce;
    // caclulate total sol balance
    const calaculateTotalSolBalance = (balance: Balance) => {
      let totalSolBalance = new BN(0);
      for (var tokenBalance of balance.tokenBalances.values()) {
        totalSolBalance = totalSolBalance.add(tokenBalance.totalBalanceSol);
      }
      return totalSolBalance;
    };

    this.transactionHistory = await getUserIndexTransactions(
      indexedTransactions,
      this.provider,
      this.balance.tokenBalances,
    );

    balance.totalSolBalance = calaculateTotalSolBalance(balance);
    return balance;
  }

  /**
   * @async
   * This method retrieves all the non-accepted UTXOs that are not part of the main balance.
   * @note
   * If the `latest` parameter is set to true (which is the default), it will sync the state of the inbox balance before returning it.
   *
   * @param {boolean} latest - A flag to indicate whether to sync the state of the inbox balance before returning it. Defaults to `true`.
   *
   * @returns {Promise<InboxBalance>} A promise that resolves to the inbox balance containing all non-accepted UTXOs.
   */
  async getUtxoInbox(latest: boolean = true): Promise<InboxBalance> {
    if (latest) {
      await this.syncState(
        false,
        this.inboxBalance,
        TRANSACTION_MERKLE_TREE_KEY,
      );
    }
    return this.inboxBalance;
  }

  /**
   * @async
   * This method retrieves the current balance of the user's account.
   * If the `latest` parameter is set to true (which is the default), it will sync the state of the balance before returning it.
   *
   * @note
   * This function checks if the necessary components such as account, provider, Poseidon hasher, Merkle Tree, and Lookup Table are initialized.
   * If any of these components are not initialized, an error will be thrown.
   *
   * @param {boolean} latest - A flag to indicate whether to sync the state of the balance before returning it. Defaults to `true`.
   *
   * @returns {Promise<Balance>} A promise that resolves to the current balance of the user's account.
   *
   * @throws {UserError} UserError:
   * - If the account or the provider is not initialized.
   * - If the Poseidon hasher, the Merkle Tree, or the Lookup Table is not initialized.
   */
  async getBalance(latest: boolean = true): Promise<Balance> {
    if (!this.account)
      throw new UserError(
        UserErrorCode.UTXOS_NOT_INITIALIZED,
        "getBalances",
        "Account not initialized",
      );
    if (!this.provider)
      throw new UserError(
        UserErrorCode.USER_ACCOUNT_NOT_INITIALIZED,
        "Provider not initialized",
      );
    if (!this.provider.poseidon)
      throw new UserError(
        TransactionParametersErrorCode.NO_POSEIDON_HASHER_PROVIDED,
        "Poseidon not initialized",
      );
    if (!this.provider.solMerkleTree)
      throw new UserError(
        ProviderErrorCode.SOL_MERKLE_TREE_UNDEFINED,
        "getBalance",
        "Merkle Tree not initialized",
      );
    if (!this.provider.lookUpTable)
      throw new UserError(
        RelayerErrorCode.LOOK_UP_TABLE_UNDEFINED,
        "getBalance",
        "Look up table not initialized",
      );

    if (latest) {
      await this.syncState(true, this.balance, TRANSACTION_MERKLE_TREE_KEY);
    }
    return this.balance;
  }

  /**
   * @async
   * This method asynchronously creates transaction parameters for a shield operation.
   * - The shield operation is the process of hiding tokens in a privacy-preserving manner.
   * - The method takes an options object with various properties for the operation.
   * - The function performs various validations including checking if a public amount is provided, if the token is defined,
   * and if the provider is set, among other validations. If the validations fail, an error is thrown.
   *
   * @param {object} options - The configuration object for the operation.
   * @param {string} options.token - The type of the token to shield ("SOL", "USDC", "USDT").
   * @param {Account} options.recipient - Optional recipient account. If not set, will shield to self.
   * @param {number | BN | string} options.publicAmountSpl - The amount of tokens to shield.
   * @param {number | BN | string} options.publicAmountSol - The amount of SOL to add to the shielded amount.
   * @param {PublicKey} options.senderTokenAccount - Optional token account to shield from, else derives ATA.
   * @param {boolean} options.minimumLamports - Optional, if set, will add minimum SOL to the shielded amount. Default is true.
   * @param {AppUtxoConfig} options.appUtxo - Optional configuration object for app-specific UTXO.
   * @param {boolean} options.mergeExistingUtxos - Optional flag to indicate whether to merge existing UTXOs. Default is true.
   * @param {Idl} options.verifierIdl - Optional, the Interface Description Language (IDL) for the verifier.
   * @param {Buffer} options.message - Optional message to include in the transaction.
   * @param {boolean} options.skipDecimalConversions - Optional flag to skip decimal conversions for public amounts. Default is false.
   * @param {Utxo} options.utxo - Optional UTXO to include in the transaction.
   *
   * @returns {Promise<TransactionParameters>} A promise that resolves to the transaction parameters for the shield operation.
   *
   * @throws {UserError} UserError:
   * - If the token is "SOL" and publicAmountSpl is provided.
   * - If both publicAmountSpl and publicAmountSol are not provided.
   * - If token is not defined but publicAmountSpl is provided.
   * - If the provider is not set.
   * - If the token is not supported or token account is defined for SOL.
   */
  async createShieldTransactionParameters({
    token,
    publicAmountSpl,
    recipient,
    publicAmountSol,
    senderTokenAccount,
    minimumLamports = true,
    appUtxo,
    mergeExistingUtxos = true,
    verifierIdl,
    message,
    skipDecimalConversions = false,
    utxo,
  }: {
    token: string;
    recipient?: Account;
    publicAmountSpl?: number | BN | string;
    publicAmountSol?: number | BN | string;
    minimumLamports?: boolean;
    senderTokenAccount?: PublicKey;
    appUtxo?: AppUtxoConfig;
    mergeExistingUtxos?: boolean;
    verifierIdl?: Idl;
    message?: Buffer;
    skipDecimalConversions?: boolean;
    utxo?: Utxo;
  }): Promise<TransactionParameters> {
    // TODO: add errors for if appUtxo appDataHash or appData, no verifierAddress
    if (publicAmountSpl && token === "SOL")
      throw new UserError(
        UserErrorCode.INVALID_TOKEN,
        "shield",
        "No public amount provided. Shield needs a public amount.",
      );
    if (!publicAmountSpl && !publicAmountSol)
      throw new UserError(
        CreateUtxoErrorCode.NO_PUBLIC_AMOUNTS_PROVIDED,
        "shield",
        "No public amounts provided. Shield needs a public amount.",
      );

    if (publicAmountSpl && !token)
      throw new UserError(
        UserErrorCode.TOKEN_UNDEFINED,
        "shield",
        "No public amounts provided. Shield needs a public amount.",
      );

    if (!this.provider)
      throw new UserError(
        UserErrorCode.PROVIDER_NOT_INITIALIZED,
        "shield",
        "Provider not set!",
      );

    let tokenCtx = TOKEN_REGISTRY.get(token);

    if (!tokenCtx)
      throw new UserError(
        UserErrorCode.TOKEN_NOT_FOUND,
        "shield",
        "Token not supported!",
      );
    if (tokenCtx.isNative && senderTokenAccount)
      throw new UserError(
        UserErrorCode.TOKEN_ACCOUNT_DEFINED,
        "shield",
        "Cannot use senderTokenAccount for SOL!",
      );
    let userSplAccount: PublicKey | undefined = undefined;
    const convertedPublicAmounts = decimalConversion({
      tokenCtx,
      skipDecimalConversions,
      publicAmountSol,
      publicAmountSpl,
      minimumLamports,
      minimumLamportsAmount: this.provider.minimumLamports,
    });
    publicAmountSol = convertedPublicAmounts.publicAmountSol
      ? convertedPublicAmounts.publicAmountSol
      : new BN(0);
    publicAmountSpl = convertedPublicAmounts.publicAmountSpl;

    if (!tokenCtx.isNative && publicAmountSpl) {
      if (senderTokenAccount) {
        userSplAccount = senderTokenAccount;
      } else {
        userSplAccount = splToken.getAssociatedTokenAddressSync(
          tokenCtx!.mint,
          this.provider!.wallet!.publicKey,
        );
      }
    }
    // TODO: add get utxos as array method
    let utxosEntries = this.balance.tokenBalances
      .get(tokenCtx.mint.toBase58())
      ?.utxos.values();
    let utxos: Utxo[] =
      utxosEntries && mergeExistingUtxos ? Array.from(utxosEntries) : [];
    let outUtxos: Utxo[] = [];
    if (recipient) {
      const amounts: BN[] = publicAmountSpl
        ? [publicAmountSol, publicAmountSpl]
        : [publicAmountSol];
      const assets = !tokenCtx.isNative
        ? [SystemProgram.programId, tokenCtx.mint]
        : [SystemProgram.programId];
      outUtxos.push(
        new Utxo({
          poseidon: this.provider.poseidon,
          assets,
          amounts,
          account: recipient,
          appDataHash: appUtxo?.appDataHash,
          verifierAddress: appUtxo?.verifierAddress,
          includeAppData: appUtxo?.includeAppData,
          appData: appUtxo?.appData,
          assetLookupTable: this.provider.lookUpTables.assetLookupTable,
          verifierProgramLookupTable:
            this.provider.lookUpTables.verifierProgramLookupTable,
        }),
      );
      // no merging of utxos when shielding to another recipient
      mergeExistingUtxos = false;
      utxos = [];
    }
    if (utxo) outUtxos.push(utxo);
    const txParams = await TransactionParameters.getTxParams({
      tokenCtx,
      action: Action.SHIELD,
      account: this.account,
      utxos,
      publicAmountSol,
      publicAmountSpl,
      userSplAccount,
      provider: this.provider,
      transactionNonce: this.balance.transactionNonce,
      appUtxo,
      verifierIdl: verifierIdl ? verifierIdl : this.verifierIdl,
      outUtxos,
      addInUtxos: recipient ? false : true,
      addOutUtxos: recipient ? false : true,
      message,
      assetLookupTable: this.provider.lookUpTables.assetLookupTable,
      verifierProgramLookupTable:
        this.provider.lookUpTables.verifierProgramLookupTable,
    });
    this.recentTransactionParameters = txParams;
    return txParams;
  }

  /**
   * @async
   * This method compiles and proves a transaction.
   * - It is primarily used for creating privacy-preserving transactions on the Solana blockchain.
   * - Before this method is invoked, the createShieldTransactionParameters method must be called to create the parameters for the transaction.
   *
   * @param {any} appParams - Optional parameters for a general application.
   *
   * @returns {Promise<Transaction>} A promise that resolves to the compiled and proven transaction.
   *
   * @throws {UserError} UserError:
   * - If the createShieldTransactionParameters method was not called before invoking this method.
   */
  async compileAndProveTransaction(appParams?: any): Promise<Transaction> {
    if (!this.recentTransactionParameters)
      throw new UserError(
        UserErrorCode.TRANSACTION_PARAMTERS_UNDEFINED,
        "compileAndProveTransaction",
        "createShieldTransactionParameters need to be executed to create parameters that be compiled and proven",
      );
    let tx = new Transaction({
      provider: this.provider,
      params: this.recentTransactionParameters,
      appParams,
    });

    await tx.compileAndProve();
    this.recentTransaction = tx;
    return tx;
  }

  /**
   * @async
   * This method approves a transaction that is waiting to be executed.
   * - Before invoking this method, createShieldTransactionParameters needs to be called to prepare the parameters for the transaction.
   * - This method is primarily used to grant permissions for the transfer of SPL tokens before a shield transaction.
   *
   * @returns {Promise<void>} A promise that resolves when the approval is completed.
   *
   * @throws {UserError} UserError:
   * - If the createShieldTransactionParameters method was not called before invoking this method.
   * - If the associated token account does not exist.
   * - If there are insufficient token balance for the transaction.
   * - If there was an error in approving the token transfer.
   */
  async approve() {
    if (!this.recentTransactionParameters)
      throw new UserError(
        UserErrorCode.TRANSACTION_PARAMTERS_UNDEFINED,
        "compileAndProveTransaction",
        "createShieldTransactionParameters need to be executed to approve spl funds prior a shield transaction",
      );
    if (
      this.recentTransactionParameters?.publicAmountSpl.gt(new BN(0)) &&
      this.recentTransactionParameters?.action === Action.SHIELD
    ) {
      let tokenBalance = await splToken.getAccount(
        this.provider.provider?.connection!,
        this.recentTransactionParameters.accounts.senderSpl!,
      );

      if (!tokenBalance)
        throw new UserError(
          UserErrorCode.ASSOCIATED_TOKEN_ACCOUNT_DOESNT_EXIST,
          "shield",
          "AssociatdTokenAccount doesn't exist!",
        );

      if (
        this.recentTransactionParameters?.publicAmountSpl.gt(
          new BN(tokenBalance.amount.toString()),
        )
      )
        throw new UserError(
          UserErrorCode.INSUFFICIENT_BAlANCE,
          "shield",
          `Insufficient token balance! ${this.recentTransactionParameters?.publicAmountSpl.toString()} bal: ${tokenBalance!
            .amount!}`,
        );

      try {
        const transaction = new SolanaTransaction().add(
          splToken.createApproveInstruction(
            this.recentTransactionParameters.accounts.senderSpl!,
            AUTHORITY,
            this.provider.wallet!.publicKey,
            this.recentTransactionParameters?.publicAmountSpl.toNumber(),
            [this.provider.wallet!.publicKey],
          ),
        );

        await this.provider.wallet!.sendAndConfirmTransaction(transaction);
        this.approved = true;
      } catch (e) {
        throw new UserError(
          UserErrorCode.APPROVE_ERROR,
          "shield",
          `Error approving token transfer! ${e}`,
        );
      }
    } else {
      this.approved = true;
    }
  }

  /**
   * @async
   * This method sends a transaction that has been compiled and approved, and waits for confirmation of the transaction.
   * @note
   * This method is primarily used to execute a shield transaction involving SPL tokens.
   *
   * @returns {Promise<string | undefined>} A promise that resolves to the transaction hash when the transaction is confirmed.
   *
   * @throws {UserError} UserError:
   * - If SPL funds are not approved before invoking this method for a shield transaction.
   * - If the transaction is not compiled and proof is not generated before calling this method.
   * - If there was an error in sending and confirming the transaction.
   */
  async sendAndConfirm() {
    if (
      this.recentTransactionParameters?.action === Action.SHIELD &&
      !this.approved
    )
      throw new UserError(
        UserErrorCode.SPL_FUNDS_NOT_APPROVED,
        "sendAndConfirmed",
        "spl funds need to be approved before a shield with spl tokens can be executed",
      );
    if (!this.recentTransaction)
      throw new UserError(
        UserErrorCode.TRANSACTION_UNDEFINED,
        "sendAndConfirmed",
        "transaction needs to be compiled and a proof generated before send.",
      );
    let txHash;
    try {
      txHash = await this.recentTransaction?.sendAndConfirmTransaction();
    } catch (e) {
      throw new UserError(
        TransactionErrorCode.SEND_TRANSACTION_FAILED,
        "shield",
        `Error in tx.sendAndConfirmTransaction! ${e}`,
      );
    }
    let transactionContainsEncryptedUtxo = false;
    this.recentTransactionParameters?.outputUtxos.map((utxo) => {
      if (utxo.account.pubkey.toString() === this.account?.pubkey.toString()) {
        transactionContainsEncryptedUtxo = true;
      }
    });
    this.balance.transactionNonce += 1;
    return txHash;
  }

  /**
   * @async
   * This method updates the Merkle tree. After updating, it resets the recent transaction parameters and approval status.
   *
   * @returns {Promise<any>} A promise that resolves when the Merkle tree update request to the relayer is complete.
   *
   * @remarks This method is primarily used to update the state of the Merkle tree after a transaction.
   * It uses the provider's relayer to update the Merkle tree and resets the transaction-related state.
   */
  async updateMerkleTree() {
    const response = await this.provider.relayer.updateMerkleTree(
      this.provider,
    );

    await this.syncState(true, this.balance, TRANSACTION_MERKLE_TREE_KEY);

    this.recentTransaction = undefined;
    this.recentTransactionParameters = undefined;
    this.approved = undefined;
    return response;
  }

  /**
   * @async
   * This method processes a shield operation.
   *
   * @param {object} params - Object containing parameters for the shield operation.
   * @param {string} params.token - The token to be shielded, e.g., "SOL", "USDC", "USDT".
   * @param {number | BN | string} [params.publicAmountSpl] - The amount to shield, e.g., 1 SOL = 1, 2 USDC = 2.
   * @param {string} [params.recipient] - Optional recipient account. If not set, the operation will shield to self.
   * @param {number | BN | string} [params.publicAmountSol] - Optional extra SOL amount to add to the shielded amount.
   * @param {PublicKey} [params.senderTokenAccount] - Optional sender's token account. If set, this account will be used to shield from, else derives an ATA.
   * @param {boolean} [params.minimumLamports=true] - Optional flag to indicate whether to use minimum lamports or not.
   * @param {AppUtxoConfig} [params.appUtxo] - Optional application UTXO configuration.
   * @param {boolean} [params.skipDecimalConversions=false] - Optional flag to skip decimal conversions.
   *
   * @returns {Promise<{txHash: string, response: any}>} - A promise that resolves to an object containing the transaction hash and the response from updating the Merkle tree.
   *
   * @remarks This method performs a shield operation, which involves:
   * - creating transaction parameters
   * - compiling and proving the transaction
   * - approving the transaction
   * - sending and confirming the transaction
   * - finally updating the Merkle tree.
   */
  async shield({
    token,
    publicAmountSpl,
    recipient,
    publicAmountSol,
    senderTokenAccount,
    minimumLamports = true,
    appUtxo,
    skipDecimalConversions = false,
  }: {
    token: string;
    recipient?: string;
    publicAmountSpl?: number | BN | string;
    publicAmountSol?: number | BN | string;
    minimumLamports?: boolean;
    senderTokenAccount?: PublicKey;
    appUtxo?: AppUtxoConfig;
    skipDecimalConversions?: boolean;
  }) {
    let recipientAccount = recipient
      ? Account.fromPubkey(recipient, this.provider.poseidon)
      : undefined;

    await this.createShieldTransactionParameters({
      token,
      publicAmountSpl,
      recipient: recipientAccount,
      publicAmountSol,
      senderTokenAccount,
      minimumLamports,
      appUtxo,
      skipDecimalConversions,
    });
    await this.compileAndProveTransaction();
    await this.approve();
    const txHash = await this.sendAndConfirm();
    const response = await this.updateMerkleTree();
    return { txHash, response };
  }

  /**
   * @async
   * This method processes an unshield operation.
   *
   * @param {object} params - Object containing parameters for the unshield operation.
   * @param {string} params.token - The token to be unshielded.
   * @param {PublicKey} [params.recipientSpl=AUTHORITY] - Recipient of the SPL token. Defaults to AUTHORITY.
   * @param {PublicKey} [params.recipientSol=AUTHORITY] - Recipient of the SOL token. Defaults to AUTHORITY.
   * @param {number | BN | string} [params.publicAmountSpl] - The amount of SPL to unshield.
   * @param {number | BN | string} [params.publicAmountSol] - The amount of SOL to unshield.
   * @param {boolean} [params.minimumLamports=true] - Optional flag to indicate whether to use minimum lamports or not.
   *
   * @returns {Promise<{txHash: string, response: any}>} - A promise that resolves to an object containing the transaction hash and the response from updating the Merkle tree.
   *
   * @remarks This method performs an unshield operation, which involves:
   * - creating transaction parameters
   * - compiling and proving the transaction
   * - sending and confirming the transaction
   * - finally updating the Merkle tree.
   */
  async unshield({
    token,
    publicAmountSpl,
    recipientSpl = AUTHORITY,
    publicAmountSol,
    recipientSol = AUTHORITY,
    minimumLamports = true,
  }: {
    token: string;
    recipientSpl?: PublicKey;
    recipientSol?: PublicKey;
    publicAmountSpl?: number | BN | string;
    publicAmountSol?: number | BN | string;
    minimumLamports?: boolean;
  }) {
    await this.createUnshieldTransactionParameters({
      token,
      publicAmountSpl,
      recipientSpl,
      publicAmountSol,
      recipientSol,
      minimumLamports,
    });

    await this.compileAndProveTransaction();
    const txHash = await this.sendAndConfirm();
    const response = await this.updateMerkleTree();
    return { txHash, response };
  }

  // TODO: add unshieldSol and unshieldSpl
  // TODO: add optional passs-in token mint
  // TODO: add pass-in mint

  /**
   * @async
   * This method prepares transaction parameters for an unshield operation. This involves:
   * - checking the token context, recipients, balances, and UTXOs.
   * - It eventually calls `TransactionParameters.getTxParams` to get the final transaction parameters.
   * - The recent transaction parameters are then updated with the result.
   *
   * @param {object} params - Object containing parameters for the unshield operation.
   * @param {string} params.token - The token to be unshielded.
   * @param {PublicKey} [params.recipientSpl=AUTHORITY] - Recipient of the SPL token. Defaults to AUTHORITY.
   * @param {PublicKey} [params.recipientSol=AUTHORITY] - Recipient of the SOL token. Defaults to AUTHORITY.
   * @param {number | BN | string} [params.publicAmountSpl] - The amount of SPL to unshield.
   * @param {number | BN | string} [params.publicAmountSol] - The amount of SOL to unshield.
   * @param {boolean} [params.minimumLamports=true] - Optional flag to indicate whether to use minimum lamports or not.
   *
   * @returns {Promise<any>} - A promise that resolves to the transaction parameters.
   *
   */
  async createUnshieldTransactionParameters({
    token,
    publicAmountSpl,
    recipientSpl = AUTHORITY,
    publicAmountSol,
    recipientSol = AUTHORITY,
    minimumLamports = true,
  }: {
    token: string;
    recipientSpl?: PublicKey;
    recipientSol?: PublicKey;
    publicAmountSpl?: number | BN | string;
    publicAmountSol?: number | BN | string;
    minimumLamports?: boolean;
  }) {
    const tokenCtx = TOKEN_REGISTRY.get(token);
    if (!tokenCtx)
      throw new UserError(
        UserErrorCode.TOKEN_NOT_FOUND,
        "shield",
        "Token not supported!",
      );

    if (!publicAmountSpl && !publicAmountSol)
      throw new UserError(
        CreateUtxoErrorCode.NO_PUBLIC_AMOUNTS_PROVIDED,
        "unshield",
        "Need to provide at least one amount for an unshield",
      );
    if (publicAmountSol && recipientSol.toBase58() == AUTHORITY.toBase58())
      throw new UserError(
        TransactionErrorCode.SOL_RECIPIENT_UNDEFINED,
        "getTxParams",
        "no recipient provided for sol unshield",
      );
    if (publicAmountSpl && recipientSpl.toBase58() == AUTHORITY.toBase58())
      throw new UserError(
        TransactionErrorCode.SPL_RECIPIENT_UNDEFINED,
        "getTxParams",
        "no recipient provided for spl unshield",
      );

    let ataCreationFee = false;

    if (!tokenCtx.isNative && publicAmountSpl) {
      let tokenBalance = await this.provider.connection?.getTokenAccountBalance(
        recipientSpl,
      );
      if (!tokenBalance?.value.uiAmount) {
        /** Signal relayer to create the ATA and charge an extra fee for it */
        ataCreationFee = true;
      }
      recipientSpl = splToken.getAssociatedTokenAddressSync(
        tokenCtx!.mint,
        recipientSpl,
      );
    }

    var _publicSplAmount: BN | undefined = undefined;
    if (publicAmountSpl) {
      _publicSplAmount = convertAndComputeDecimals(
        publicAmountSpl,
        tokenCtx.decimals,
      );
    }

    // if no sol amount by default min amount if disabled 0
    const _publicSolAmount = publicAmountSol
      ? convertAndComputeDecimals(publicAmountSol, new BN(1e9))
      : minimumLamports
      ? this.provider.minimumLamports
      : new BN(0);
    let utxosEntries = this.balance.tokenBalances
      .get(tokenCtx.mint.toBase58())
      ?.utxos.values();
    let solUtxos = this.balance.tokenBalances
      .get(SystemProgram.programId.toBase58())
      ?.utxos.values();
    let utxosEntriesSol: Utxo[] =
      solUtxos && !tokenCtx.isNative ? Array.from(solUtxos) : new Array<Utxo>();

    let utxos: Utxo[] = utxosEntries
      ? Array.from([...utxosEntries, ...utxosEntriesSol])
      : [];

    const txParams = await TransactionParameters.getTxParams({
      tokenCtx,
      publicAmountSpl: _publicSplAmount,
      action: Action.UNSHIELD,
      account: this.account,
      utxos,
      publicAmountSol: _publicSolAmount,
      recipientSol: recipientSol,
      recipientSplAddress: recipientSpl,
      provider: this.provider,
      relayer: this.provider.relayer,
      ataCreationFee,
      transactionNonce: this.balance.transactionNonce,
      appUtxo: this.appUtxoConfig,
      verifierIdl: IDL_VERIFIER_PROGRAM_ZERO,
      assetLookupTable: this.provider.lookUpTables.assetLookupTable,
      verifierProgramLookupTable:
        this.provider.lookUpTables.verifierProgramLookupTable,
    });
    this.recentTransactionParameters = txParams;
    return txParams;
  }

  // TODO: replace recipient with recipient light publickey
  /**
   * @async
   * This method initiates a shielded transfer operation.
   * - It first checks if the recipient is provided
   * - Then creates transaction parameters using the `createTransferTransactionParameters` method.
   * - It then calls `transactWithParameters` with the obtained parameters.
   *
   * @param {object} params - Object containing parameters for the transfer operation.
   * @param {string} params.token - The token to be transferred.
   * @param {string} params.recipient - The recipient of the transfer.
   * @param {BN | number | string} [params.amountSpl] - The amount of SPL tokens to transfer.
   * @param {BN | number | string} [params.amountSol] - The amount of SOL tokens to transfer.
   * @param {AppUtxoConfig} [params.appUtxo] - Configuration for the application UTXO.
   *
   * @returns {Promise<any>} - A promise that resolves to the result of the transaction with the parameters.
   *
   */
  async transfer({
    token,
    recipient,
    amountSpl,
    amountSol,
    appUtxo,
  }: {
    token: string;
    amountSpl?: BN | number | string;
    amountSol?: BN | number | string;
    recipient: string;
    appUtxo?: AppUtxoConfig;
  }) {
    if (!recipient)
      throw new UserError(
        UserErrorCode.SHIELDED_RECIPIENT_UNDEFINED,
        "transfer",
        "No shielded recipient provided for transfer.",
      );
    let recipientAccount = Account.fromPubkey(
      recipient,
      this.provider.poseidon,
    );

    let txParams = await this.createTransferTransactionParameters({
      token,
      recipient: recipientAccount,
      amountSpl,
      amountSol,
      appUtxo,
    });
    return this.transactWithParameters({ txParams });
  }

  // TODO: add separate lookup function for users.
  // TODO: add account parsing from and to string which is concat shielded pubkey and encryption key

  /**
   * @async
   *  This method prepares the transaction parameters for a shielded transfer operation .i.e transfer method.
   * - It first checks if at least one amount is provided and if the token is defined.
   * - It then retrieves the token context, converts the amounts if required, and creates output UTXOs.
   * - Finally, it retrieves the input UTXOs and creates transaction parameters using the `getTxParams` method of `TransactionParameters`.
   *
   * @param {object} params - Object containing parameters for the transfer operation.
   * @param {string} [params.token] - The token to be transferred.
   * @param {BN | number | string} [params.amountSpl] - The amount of SPL tokens to transfer.
   * @param {BN | number | string} [params.amountSol] - The amount of SOL tokens to transfer.
   * @param {Account} [params.recipient] - The recipient of the transfer.
   * @param {AppUtxoConfig} [params.appUtxo] - Configuration for the application UTXO.
   * @param {Buffer} [params.message] - Buffer containing a message to be included in the transaction.
   * @param {Utxo} [params.outUtxo] - An UTXO to be included in the transaction.
   * @param {Idl} [params.verifierIdl] - An Interface Definition Language (IDL) for the verifier.
   * @param {boolean} [params.skipDecimalConversions] - Whether to skip decimal conversions.
   * @param {boolean} [params.addInUtxos] - Whether to add in UTXOs to the transaction.
   *
   * @returns {Promise<any>} - A promise that resolves to the transaction parameters.
   *
   */
  async createTransferTransactionParameters({
    token,
    recipient,
    amountSpl,
    amountSol,
    appUtxo,
    message,
    inUtxos,
    outUtxos,
    verifierIdl,
    skipDecimalConversions,
    addInUtxos = true,
    addOutUtxos = true,
  }: {
    token?: string;
    amountSpl?: BN | number | string;
    amountSol?: BN | number | string;
    recipient?: Account;
    appUtxo?: AppUtxoConfig;
    message?: Buffer;
    inUtxos?: Utxo[];
    outUtxos?: Utxo[];
    verifierIdl?: Idl;
    skipDecimalConversions?: boolean;
    addInUtxos?: boolean;
    addOutUtxos?: boolean;
  }) {
    if (!amountSol && !amountSpl && !outUtxos && !inUtxos)
      throw new UserError(
        UserErrorCode.NO_AMOUNTS_PROVIDED,
        "createTransferTransactionParameters",
        "At least one amount should be provided for a transfer.",
      );
    if ((!token && outUtxos) || inUtxos) {
      if (outUtxos)
        token = TOKEN_PUBKEY_SYMBOL.get(outUtxos[0].assets[1].toBase58());
      if (inUtxos)
        token = TOKEN_PUBKEY_SYMBOL.get(inUtxos[0].assets[1].toBase58());
    }
    if (!token)
      throw new UserError(
        UserErrorCode.TOKEN_UNDEFINED,
        "createTransferTransactionParameters",
      );

    const tokenCtx = TOKEN_REGISTRY.get(token);
    if (!tokenCtx)
      throw new UserError(
        UserErrorCode.TOKEN_NOT_FOUND,
        "createTransferTransactionParameters",
        "Token not supported!",
      );
    const convertedPublicAmounts = decimalConversion({
      tokenCtx,
      skipDecimalConversions,
      publicAmountSol: amountSol,
      publicAmountSpl: amountSpl,
      minimumLamportsAmount: this.provider.minimumLamports,
    });
    var parsedSolAmount = convertedPublicAmounts.publicAmountSol
      ? convertedPublicAmounts.publicAmountSol
      : new BN(0);
    var parsedSplAmount = convertedPublicAmounts.publicAmountSpl
      ? convertedPublicAmounts.publicAmountSpl
      : new BN(0);

    // var parsedSplAmount: BN = amountSpl
    //   ? new BN(amountSpl.toString())
    //   : new BN(0);
    // if (!skipDecimalConversions && amountSpl && tokenCtx) {
    //   parsedSplAmount = convertAndComputeDecimals(amountSpl, tokenCtx.decimals);
    // }
    // // if no sol amount by default min amount if disabled 0
    // var parsedSolAmount: BN = amountSol
    //   ? new BN(amountSol.toString())
    //   : new BN(0);
    // if (!skipDecimalConversions && amountSol) {
    //   parsedSolAmount = convertAndComputeDecimals(amountSol, new BN(1e9));
    // }

    if (recipient && !tokenCtx)
      throw new UserError(
        UserErrorCode.SHIELDED_RECIPIENT_UNDEFINED,
        "createTransferTransactionParameters",
      );

    let _outUtxos: Utxo[] = [];
    if (recipient) {
      _outUtxos = createRecipientUtxos({
        recipients: [
          {
            mint: tokenCtx.mint,
            account: recipient,
            solAmount: parsedSolAmount,
            splAmount: parsedSplAmount,
            appUtxo,
          },
        ],
        poseidon: this.provider.poseidon,
        assetLookupTable: this.provider.lookUpTables.assetLookupTable,
        verifierProgramLookupTable:
          this.provider.lookUpTables.verifierProgramLookupTable,
      });
    }

    if (outUtxos) _outUtxos = [..._outUtxos, ...outUtxos];

    let utxos: Utxo[] = [];

    let solUtxos = this.balance.tokenBalances
      .get(SystemProgram.programId.toBase58())
      ?.utxos.values();
    let utxosEntriesSol: Utxo[] =
      solUtxos && token !== "SOL" ? Array.from(solUtxos) : new Array<Utxo>();

    let utxosEntries = this.balance.tokenBalances
      .get(tokenCtx.mint.toBase58())
      ?.utxos.values();
    utxos = utxosEntries
      ? Array.from([...utxosEntries, ...utxosEntriesSol])
      : [];

    if (!tokenCtx.isNative && !utxosEntries)
      throw new UserError(
        UserErrorCode.INSUFFICIENT_BAlANCE,
        "createTransferTransactionParamters",
        `Balance does not have any utxos of ${token}`,
      );

    const txParams = await TransactionParameters.getTxParams({
      tokenCtx,
      action: Action.TRANSFER,
      account: this.account,
      utxos,
      inUtxos,
      outUtxos: _outUtxos,
      provider: this.provider,
      relayer: this.provider.relayer,
      transactionNonce: this.balance.transactionNonce,
      verifierIdl: verifierIdl ? verifierIdl : this.verifierIdl,
      appUtxo: this.appUtxoConfig,
      message,
      addInUtxos,
      addOutUtxos,
      assetLookupTable: this.provider.lookUpTables.assetLookupTable,
      verifierProgramLookupTable:
        this.provider.lookUpTables.verifierProgramLookupTable,
    });
    this.recentTransactionParameters = txParams;
    return txParams;
  }

  /**
   * @async
   * This method performs a transaction using the provided transaction parameters.
   * - It first sets the recent transaction parameters, compiles and proves the transaction, approves it, sends it and confirms it, and then updates the Merkle tree.
   * - The method returns the transaction hash and the response from updating the Merkle tree.
   *
   * @param {object} params - Object containing the transaction parameters.
   * @param {TransactionParameters} params.txParams - The parameters for the transaction.
   * @param {any} [params.appParams] - Additional parameters for the application.
   *
   * @returns {Promise<object>} - A promise that resolves to an object containing the transaction hash and the response from updating the Merkle tree.
   *
   */
  async transactWithParameters({
    txParams,
    appParams,
  }: {
    txParams: TransactionParameters;
    appParams?: any;
  }) {
    this.recentTransactionParameters = txParams;

    await this.compileAndProveTransaction(appParams);
    await this.approve();
    const txHash = await this.sendAndConfirm();
    const response = await this.updateMerkleTree();
    return { txHash, response };
  }

  /**
   * @async
   * This method is intended to perform a transaction using the provided input and output UTXOs and the specified action.
   *
   * @note
   * Currently, this method is not implemented and will throw an error if called.
   *
   * @param {object} params - Object containing the transaction parameters.
   * @param {Utxo[]} params.inUtxos - The UTXOs that are being spent in this transaction.
   * @param {Utxo[]} params.outUtxos - The UTXOs that are being created by this transaction.
   * @param {Action} params.action - The type of action being performed by this transaction (e.g., TRANSFER, SHIELD, UNSHIELD).
   * @param {string[]} params.inUtxoCommitments - The commitments of the input UTXOs.
   *
   */
  async transactWithUtxos({
    inUtxos,
    outUtxos,
    action,
    inUtxoCommitments,
  }: {
    inUtxos: Utxo[];
    outUtxos: Utxo[];
    action: Action;
    inUtxoCommitments: string[];
  }) {
    throw new Error("Unimplemented");
  }

  /**
   * @async
   * This method asynchronously initializes a User instance.
   * - This method initializes a User instance by using the provided Light provider, optional user seed and UTXOs, application UTXO configuration, and account.
   * - If no seed is provided, a signature prompt will appear for login. If no Poseidon function is provided, it will build one. If no account is provided, it will create a new one with the given Poseidon function and seed.
   * - After initializing the user, it will retrieve the user's balance.
   *
   * @param {object} params - Object containing the initialization parameters.
   * @param {Provider} params.provider - The Light provider to be used.
   * @param {string} [params.seed] - Optional user seed to instantiate from. If the seed is supplied, it skips the log-in signature prompt.
   * @param {Utxo[]} [params.utxos] - Optional user UTXOs (Unspent Transaction Outputs) to instantiate from.
   * @param {AppUtxoConfig} [params.appUtxoConfig] - Optional application UTXO configuration.
   * @param {Account} [params.account] - Optional account to be used.
   *
   * @returns {Promise<User>} - Returns a Promise that resolves to the initialized User instance.
   *
   * @throws {UserError} UserError:
   * - Throws a UserError if no wallet is provided or if there is an error while loading the user.
   *
   */
  static async init({
    provider,
    seed,
    utxos,
    appUtxoConfig,
    account,
  }: {
    provider: Provider;
    seed?: string;
    utxos?: Utxo[];
    appUtxoConfig?: AppUtxoConfig;
    account?: Account;
  }): Promise<any> {
    try {
      if (!seed) {
        if (provider.wallet) {
          const signature: Uint8Array = await provider.wallet.signMessage(
            message,
          );
          seed = new anchor.BN(signature).toString();
        } else {
          throw new UserError(
            UserErrorCode.NO_WALLET_PROVIDED,
            "load",
            "No payer or browser wallet provided",
          );
        }
      }
      if (!provider.poseidon) {
        provider.poseidon = await circomlibjs.buildPoseidonOpt();
      }
      if (!account) {
        account = new Account({
          poseidon: provider.poseidon,
          seed,
        });
      }
      const user = new User({ provider, appUtxoConfig, account });

      await user.getBalance();

      return user;
    } catch (e) {
      throw new UserError(
        UserErrorCode.LOAD_ERROR,
        "load",
        `Error while loading user! ${e}`,
      );
    }
  }

  // TODO: how do we handle app utxos?, some will not be able to be accepted we can only mark these as accepted

  /**
   * @async
   * Merges all UTXOs (Unspent Transaction Outputs) for a specific asset.
   * - This method retrieves the UTXO inbox and balance of the user for the specified asset.
   * - If the inbox for the asset is empty, it throws an error.
   * - Otherwise, it retrieves the UTXOs from the balance and inbox. If the total number of UTXOs is greater than 10, it selects only the first 10.
   * - It then prepares transaction parameters for a TRANSFER action and performs the transaction.
   * - Finally, it updates the Merkle tree and returns the transaction hash and the response of the update.
   *
   * @param {PublicKey} asset - The public key of the asset whose UTXOs are to be merged.
   * @param {boolean} [latest=true] - Optional parameter indicating whether to get the latest UTXO inbox and balance. Defaults to true.
   *
   * @returns {Promise<object>} - Returns a Promise that resolves to an object containing the transaction hash and the response of the update to the Merkle tree.
   *
   */
  async mergeAllUtxos(asset: PublicKey, latest: boolean = true) {
    await this.getUtxoInbox(latest);
    await this.getBalance(latest);
    let inboxTokenBalance: TokenUtxoBalance | undefined =
      this.inboxBalance.tokenBalances.get(asset.toString());
    if (!inboxTokenBalance)
      throw new UserError(
        UserErrorCode.EMPTY_INBOX,
        "mergeAllUtxos",
        `for asset ${asset} the utxo inbox is empty`,
      );

    let utxosEntries = this.balance.tokenBalances
      .get(asset.toBase58())
      ?.utxos.values();
    let inboxUtxosEntries = Array.from(inboxTokenBalance.utxos.values());

    if (inboxUtxosEntries.length == 0)
      throw new UserError(
        UserErrorCode.EMPTY_INBOX,
        "mergeAllUtxos",
        `for asset ${asset} the utxo inbox is empty`,
      );
    let assetIndex =
      asset.toBase58() === SystemProgram.programId.toBase58() ? 0 : 1;
    // sort inbox utxos descending
    inboxUtxosEntries.sort(
      (a, b) =>
        b.amounts[assetIndex].toNumber() - a.amounts[assetIndex].toNumber(),
    );

    let inUtxos: Utxo[] = utxosEntries
      ? Array.from([...utxosEntries, ...inboxUtxosEntries])
      : Array.from(inboxUtxosEntries);

    if (inUtxos.length > 10) {
      inUtxos = inUtxos.slice(0, 10);
    }

    let txParams = await TransactionParameters.getTxParams({
      tokenCtx: inboxTokenBalance.tokenData,
      action: Action.TRANSFER,
      provider: this.provider,
      transactionNonce: this.balance.transactionNonce,
      inUtxos,
      addInUtxos: false,
      addOutUtxos: true,
      account: this.account,
      mergeUtxos: true,
      relayer: this.provider.relayer,
      verifierIdl: IDL_VERIFIER_PROGRAM_ONE,
      assetLookupTable: this.provider.lookUpTables.assetLookupTable,
      verifierProgramLookupTable:
        this.provider.lookUpTables.verifierProgramLookupTable,
    });
    this.recentTransactionParameters = txParams;
    await this.compileAndProveTransaction();
    const txHash = await this.sendAndConfirm();
    const response = await this.updateMerkleTree();
    return { txHash, response };
  }

  // TODO: how do we handle app utxos?, some will not be able to be accepted we can only mark these as accepted

  /**
   * @async
   * Performs a shielded transfer to self, merging UTXOs in the process.
   * - This method retrieves the UTXO inbox and balance of the user for the specified asset.
   * - It validates the commitments provided, and if any are not found in the UTXO inbox, it throws an error.
   * - It then retrieves the UTXOs from the balance and the commitments.
   * - If the total number of UTXOs is greater than 10, it throws an error.
   * - Otherwise, it prepares transaction parameters for a TRANSFER action and performs the transaction.
   * - Finally, it updates the Merkle tree and returns the transaction hash and the response of the update.
   * @param {string[]} commitments - An array of commitment strings for merging.
   * @param {PublicKey} asset - The public key of the asset whose UTXOs are to be merged.
   * @param {boolean} [latest=false] - Optional parameter indicating whether to get the latest UTXO inbox and balance. Defaults to false.
   *
   * @returns {Promise<object>} - Returns a Promise that resolves to an object containing the transaction hash and the response of the update to the Merkle tree.
   *
   */
  async mergeUtxos(
    commitments: string[],
    asset: PublicKey,
    latest: boolean = false,
  ) {
    if (commitments.length == 0)
      throw new UserError(
        UserErrorCode.NO_COMMITMENTS_PROVIDED,
        "mergeAllUtxos",
        `No commitmtents for merging specified ${asset}`,
      );

    await this.getUtxoInbox(latest);
    await this.getBalance(latest);
    let inboxTokenBalance: TokenUtxoBalance | undefined =
      this.inboxBalance.tokenBalances.get(asset.toString());
    if (!inboxTokenBalance)
      throw new UserError(
        UserErrorCode.EMPTY_INBOX,
        "mergeAllUtxos",
        `for asset ${asset} the utxo inbox is empty`,
      );

    let utxosEntries = this.balance.tokenBalances
      .get(asset.toBase58())
      ?.utxos.values();

    let commitmentUtxos: Utxo[] = [];
    for (var commitment of commitments) {
      let utxo = inboxTokenBalance.utxos.get(commitment);
      if (!utxo)
        throw new UserError(
          UserErrorCode.COMMITMENT_NOT_FOUND,
          "mergeUtxos",
          `commitment ${commitment} is it of asset ${asset} ?`,
        );
      commitmentUtxos.push(utxo);
    }

    let inUtxos: Utxo[] = utxosEntries
      ? Array.from([...utxosEntries, ...commitmentUtxos])
      : Array.from(commitmentUtxos);

    if (inUtxos.length > 10) {
      throw new UserError(
        UserErrorCode.TOO_MANY_COMMITMENTS,
        "mergeUtxos",
        `too many commitments provided to merge at once provided ${
          commitmentUtxos.length
        }, number of existing utxos ${
          Array.from(utxosEntries ? utxosEntries : []).length
        } > 10 (can only merge 10 utxos in one transaction)`,
      );
    }

    let txParams = await TransactionParameters.getTxParams({
      tokenCtx: inboxTokenBalance.tokenData,
      action: Action.TRANSFER,
      provider: this.provider,
      transactionNonce: this.balance.transactionNonce,
      inUtxos,
      addInUtxos: false,
      addOutUtxos: true,
      account: this.account,
      mergeUtxos: true,
      relayer: this.provider.relayer,
      verifierIdl: IDL_VERIFIER_PROGRAM_ONE,
      assetLookupTable: this.provider.lookUpTables.assetLookupTable,
      verifierProgramLookupTable:
        this.provider.lookUpTables.verifierProgramLookupTable,
    });
    this.recentTransactionParameters = txParams;
    await this.compileAndProveTransaction();
    const txHash = await this.sendAndConfirm();
    const response = await this.updateMerkleTree();
    return { txHash, response };
  }

  /**
   * @async
   * This method retrieves the transaction history of the user.
   * - If the 'latest' parameter is true, it first gets the latest balance of the user.
   * - The transaction history is then retrieved and returned.
   * - If any error occurs during this process, it is caught and a UserError is thrown with a specific error code.
   *
   * @param {boolean} [latest=true] - Optional parameter indicating whether to get the latest balance. Defaults to true.
   *
   * @returns {Promise<IndexedTransaction[]>} - Returns a Promise that resolves to an array of indexed transactions.
   *
   */
  async getTransactionHistory(
    latest: boolean = true,
  ): Promise<IndexedTransaction[]> {
    try {
      if (latest) {
        await this.getBalance(true);
      }
      return this.transactionHistory!;
    } catch (error) {
      throw new UserError(
        TransactionErrorCode.GET_USER_TRANSACTION_HISTORY_FAILED,
        "getLatestTransactionHistory",
        `Error while getting user transaction history ! ${error}`,
      );
    }
  }

  // TODO: add proof-of-origin call.
  // TODO: merge with getUtxoStatus?

  /**
   * Retrieves the status of the Unspent Transaction Output (UTXO).
   *
   * @remarks
   * - This method is intended to retrieve the status of the Unspent Transaction Output (UTXO).
   * - However, it's currently not implemented and will throw an error when called.
   * - Future implementations should provide the functionality for fetching and returning the UTXO status.
   */
  getUtxoStatus() {
    throw new Error("not implemented yet");
  }

  /**
   * Adds Unspent Transaction Outputs (UTXOs) to the user.
   *
   * @remarks
   * - This method is intended to add UTXOs to the user.
   * - This method may also be responsible for fetching UTXOs such that the user object is not occupied while fetching.
   * - Furthermore, the implementation could include calculating the user's privacy score for unshielded transactions.
   * - However, it's currently not implemented and will throw an error when called.
   * - Future implementations should provide the functionality for adding UTXOs.
   */
  addUtxos() {
    throw new Error("not implemented yet");
  }

  /**
   * @async
   * This method is used to create the transaction parameters for storing application UTXO.
   * - It performs a series of checks and operations to generate the necessary parameters.
   * - The method can handle both shielding and transfer actions.
   *
   * @param token - The token symbol.
   * @param amountSol - The amount of SOL tokens.
   * @param amountSpl - The amount of SPL tokens.
   * @param minimumLamports - A flag to indicate whether to include minimum lamports.
   * @param senderTokenAccount - The public key of the sender's token account.
   * @param recipientPublicKey - The public key of the recipient.
   * @param appUtxo - The application UTXO.
   * @param stringUtxo - The string representation of the UTXO.
   * @param action - The action to be performed (Action.SHIELD or Action.TRANSFER).
   * @param appUtxoConfig - The configuration of the application UTXO.
   * @param skipDecimalConversions - A flag to skip decimal conversions.
   *
   * @throws {UserError} UserError:
   * - Throws an error if the required parameters are not provided or if the max storage message size is exceeded.
   *
   * @returns Promise which resolves to the created transaction parameters.
   *
   */
  async createStoreAppUtxoTransactionParameters({
    token,
    amountSol,
    amountSpl,
    minimumLamports,
    senderTokenAccount,
    recipientPublicKey,
    appUtxo,
    stringUtxo,
    action,
    appUtxoConfig,
    skipDecimalConversions = false,
  }: {
    token?: string;
    amountSol?: BN;
    amountSpl?: BN;
    minimumLamports?: boolean;
    senderTokenAccount?: PublicKey;
    recipientPublicKey?: string;
    appUtxo?: Utxo;
    stringUtxo?: string;
    action: Action;
    appUtxoConfig?: AppUtxoConfig;
    skipDecimalConversions?: boolean;
  }) {
    if (!appUtxo) {
      if (appUtxoConfig) {
        if (!token)
          throw new UserError(
            UserErrorCode.TOKEN_UNDEFINED,
            "createStoreAppUtxoTransactionParameters",
          );
        if (!amountSol)
          throw new UserError(
            CreateUtxoErrorCode.PUBLIC_SOL_AMOUNT_UNDEFINED,
            "createStoreAppUtxoTransactionParameters",
          );
        if (!amountSpl)
          throw new UserError(
            CreateUtxoErrorCode.PUBLIC_SPL_AMOUNT_UNDEFINED,
            "createStoreAppUtxoTransactionParameters",
          );
        const tokenCtx = TOKEN_REGISTRY.get(token);
        if (!tokenCtx)
          throw new UserError(
            UserErrorCode.INVALID_TOKEN,
            "createStoreAppUtxoTransactionParameters",
          );

        appUtxo = new Utxo({
          poseidon: this.provider.poseidon,
          amounts: [amountSol, amountSpl],
          assets: [SystemProgram.programId, tokenCtx.mint],
          ...appUtxoConfig,
          account: recipientPublicKey
            ? Account.fromPubkey(recipientPublicKey, this.provider.poseidon)
            : this.account,
          verifierProgramLookupTable:
            this.provider.lookUpTables.verifierProgramLookupTable,
          assetLookupTable: this.provider.lookUpTables.assetLookupTable,
        });
      } else if (stringUtxo) {
        appUtxo = Utxo.fromString(
          stringUtxo,
          this.provider.poseidon,
          this.provider.lookUpTables.assetLookupTable,
          this.provider.lookUpTables.verifierProgramLookupTable,
        );
      } else {
        throw new UserError(
          UserErrorCode.APP_UTXO_UNDEFINED,
          "createStoreAppUtxoTransactionParameters",
          "invalid parameters to generate app utxo",
        );
      }
    } else {
      skipDecimalConversions = true;
    }
    if (!appUtxo)
      throw new UserError(
        UserErrorCode.APP_UTXO_UNDEFINED,
        "createStoreAppUtxoTransactionParameters",
        `app utxo is undefined or could not generate one from provided parameters`,
      );

    if (!token) {
      const utxoAsset =
        appUtxo.amounts[1].toString() === "0"
          ? new PublicKey(0).toBase58()
          : appUtxo.assets[1].toBase58();
      token = TOKEN_PUBKEY_SYMBOL.get(utxoAsset);
    }

    if (!token)
      throw new UserError(
        UserErrorCode.TOKEN_UNDEFINED,
        "createStoreAppUtxoTransactionParameters",
      );

    const message = Buffer.from(
      await appUtxo.encrypt(
        this.provider.poseidon,
        MESSAGE_MERKLE_TREE_KEY,
        0,
        false,
      ),
    );

    if (message.length > MAX_MESSAGE_SIZE)
      throw new UserError(
        UserErrorCode.MAX_STORAGE_MESSAGE_SIZE_EXCEEDED,
        "storeData",
        `${message.length}/${MAX_MESSAGE_SIZE}`,
      );
    appUtxo.includeAppData = false;
    if (action === Action.SHIELD) {
      if (!amountSol)
        amountSol =
          appUtxo.amounts[0].toString() === "0"
            ? undefined
            : appUtxo.amounts[0];
      if (!amountSpl)
        amountSpl =
          appUtxo.amounts[1].toString() === "0"
            ? undefined
            : appUtxo.amounts[1];

      return this.createShieldTransactionParameters({
        token,
        publicAmountSol: amountSol,
        publicAmountSpl: amountSpl,
        senderTokenAccount,
        minimumLamports,
        message,
        verifierIdl: IDL_VERIFIER_PROGRAM_STORAGE,
        skipDecimalConversions,
        utxo: appUtxo,
      });
    } else {
      return this.createTransferTransactionParameters({
        message,
        verifierIdl: IDL_VERIFIER_PROGRAM_STORAGE,
        token,
        recipient: recipientPublicKey
          ? Account.fromPubkey(recipientPublicKey, this.provider.poseidon)
          : !appUtxo
          ? this.account
          : undefined,
        amountSpl,
        amountSol,
        outUtxos: [appUtxo],
        appUtxo: appUtxoConfig,
      });
    }
  }

  // TODO: group shield parameters into type
  // TODO: group transfer parameters into type

  /**
   * @async
   * This method is used to store the application UTXO.
   * - It creates the transaction parameters using the provided parameters.
   * - Then performs the transaction with the created parameters.
   * - The method can handle both shielding and transfer actions.
   *
   * @param token - The token symbol.
   * @param amountSol - The amount of SOL tokens.
   * @param amountSpl - The amount of SPL tokens.
   * @param minimumLamports - A flag to indicate whether to include minimum lamports.
   * @param senderTokenAccount - The public key of the sender's token account.
   * @param recipientPublicKey - The public key of the recipient.
   * @param appUtxo - The application UTXO.
   * @param stringUtxo - The string representation of the UTXO.
   * @param action - The action to be performed (Action.SHIELD or Action.TRANSFER).
   * @param appUtxoConfig - The configuration of the application UTXO.
   * @param skipDecimalConversions - A flag to skip decimal conversions.
   *
   * @returns Promise which resolves to the response of the transaction with the created parameters.
   *
   */
  async storeAppUtxo({
    token,
    amountSol,
    amountSpl,
    minimumLamports,
    senderTokenAccount,
    recipientPublicKey,
    appUtxo,
    stringUtxo,
    action,
    appUtxoConfig,
    skipDecimalConversions = false,
  }: {
    token?: string;
    amountSol?: BN;
    amountSpl?: BN;
    minimumLamports?: boolean;
    senderTokenAccount?: PublicKey;
    recipientPublicKey?: string;
    appUtxo?: Utxo;
    stringUtxo?: string;
    action: Action;
    appUtxoConfig?: AppUtxoConfig;
    skipDecimalConversions?: boolean;
  }) {
    let txParams = await this.createStoreAppUtxoTransactionParameters({
      token,
      amountSol,
      amountSpl,
      minimumLamports,
      senderTokenAccount,
      recipientPublicKey,
      appUtxo,
      stringUtxo,
      action,
      appUtxoConfig,
      skipDecimalConversions,
    });

    return this.transactWithParameters({ txParams });
  }

  // TODO: add storage transaction nonce to rotate keypairs
  /**
   * This method is used to sync the storage of the user account.
   * - It gets all transactions of the storage verifier.
   * - Filters for the ones including noop program.
   * - Then it builds the merkle tree and checks versus root on-chain.
   *
   * @note - The method tries to decrypt each transaction and adds it to the appUtxos or decrypted data map.
   *
   * @param idl - The Interface Description Language (IDL) for the program.
   * @param aes - A flag indicating whether to use AES for decryption. Default is true.
   * @param merkleTree - The public key of the Merkle tree.
   *
   * @returns A promise which resolves to the balances of the programs after synchronization.
   *
   */
  async syncStorage(
    idl: anchor.Idl,
    aes: boolean = true,
    merkleTree?: PublicKey,
  ) {
    if (!aes) return undefined;
    // TODO: move to relayer
    // TODO: implement the following
    /**
     * get all transactions of the storage verifier and filter for the ones including noop program
     * build merkle tree and check versus root onchain
     * mark as cleartext and as decrypted with the first byte
     * [
     *  1 bytes: encrypted or cleartext 1 byte,
     *  32bytes:  encryptionAlgo/Mode,
     *  remaining message
     * ]
     */
    const indexedTransactions =
      await this.provider.relayer.getIndexedTransactions(
        this.provider.provider!.connection,
      );
    await this.provider.latestMerkleTree(indexedTransactions);

    const indexedStorageVerifierTransactionsFiltered =
      indexedTransactions.filter((indexedTransaction) => {
        return indexedTransaction.message.length !== 0;
      });
    // /**
    //  * - match first 8 bytes against account discriminator for every appIdl that is cached in the user class
    //  * TODO: in case we don't have it we should get the Idl from the verifierAddress
    //  * @param bytes
    //  */
    // const selectAppDataIdl = (bytes: Uint8Array) => {};

    /**
     * - aes: boolean = true
     * - decrypt storage verifier
     */
    const decryptIndexStorage = async (
      indexedTransactions: IndexedTransaction[],
      assetLookupTable: string[],
      verifierProgramLookupTable: string[],
    ) => {
      var decryptedStorageUtxos: Utxo[] = [];
      var spentUtxos: Utxo[] = [];
      for (const data of indexedTransactions) {
        let decryptedUtxo = null;
        var index = data.firstLeafIndex.toNumber();
        for (var [leafIndex, leaf] of data.leaves.entries()) {
          try {
            decryptedUtxo = await Utxo.decrypt({
              poseidon: this.provider.poseidon,
              account: this.account,
              encBytes: Uint8Array.from(data.message),
              appDataIdl: idl,
              transactionNonce: 0,
              aes: true,
              index: index,
              commitment: Uint8Array.from(leaf),
              merkleTreePdaPublicKey: MESSAGE_MERKLE_TREE_KEY,
              compressed: false,
              verifierProgramLookupTable,
              assetLookupTable,
            });
            if (decryptedUtxo !== null) {
              // const nfExists = await checkNfInserted([{isSigner: false, isWritatble: false, pubkey: Transaction.getNullifierPdaPublicKey(data.nullifiers[leafIndex], TRANSACTION_MERKLE_TREE_KEY)}], this.provider.provider?.connection!)
              const nfExists = await fetchNullifierAccountInfo(
                decryptedUtxo.getNullifier(this.provider.poseidon)!,
                this.provider.provider?.connection!,
              );
              if (!nfExists) {
                decryptedStorageUtxos.push(decryptedUtxo);
              } else {
                spentUtxos.push(decryptedUtxo);
              }
            }
            index++;
          } catch (e) {
            if (
              !(e instanceof UtxoError) ||
              e.code !== "INVALID_APP_DATA_IDL"
            ) {
              throw e;
            }
          }
        }
      }
      return { decryptedStorageUtxos, spentUtxos };
    };

    if (!this.account.aesSecret)
      throw new UserError(AccountErrorCode.AES_SECRET_UNDEFINED, "syncStorage");

    const { decryptedStorageUtxos, spentUtxos } = await decryptIndexStorage(
      indexedStorageVerifierTransactionsFiltered,
      this.provider.lookUpTables.assetLookupTable,
      this.provider.lookUpTables.verifierProgramLookupTable,
    );

    for (var utxo of decryptedStorageUtxos) {
      const verifierAddress = utxo.verifierAddress.toBase58();
      if (!this.balance.programBalances.get(verifierAddress)) {
        this.balance.programBalances.set(
          verifierAddress,
          new ProgramUtxoBalance(utxo.verifierAddress, idl),
        );
      }
      this.balance.programBalances
        .get(verifierAddress)!
        .addUtxo(utxo.getCommitment(this.provider.poseidon), utxo, "utxos");
    }

    for (var utxo of spentUtxos) {
      const verifierAddress = utxo.verifierAddress.toBase58();
      if (!this.balance.programBalances.get(verifierAddress)) {
        this.balance.programBalances.set(
          verifierAddress,
          new ProgramUtxoBalance(utxo.verifierAddress, idl),
        );
      }
      this.balance.programBalances
        .get(verifierAddress)!
        .addUtxo(
          utxo.getCommitment(this.provider.poseidon),
          utxo,
          "spentUtxos",
        );
    }
    for (var [program, programBalance] of this.balance.programBalances) {
      for (var [token, tokenBalance] of programBalance.tokenBalances) {
        for (var [key, utxo] of tokenBalance.utxos) {
          let nullifierAccountInfo = await fetchNullifierAccountInfo(
            utxo.getNullifier(this.provider.poseidon)!,
            this.provider.provider!.connection,
          );
          if (nullifierAccountInfo !== null) {
            tokenBalance.movetToSpentUtxos(key);
          }
        }
      }
    }
    return this.balance.programBalances;
  }

  /**
   * Returns an array of all UTXOs (unspent transaction outputs) across all tokens in the balance.
   * - This method aggregates all UTXOs across all tokens stored in the balance of the user's account.
   * - It's a utility method that can be used whenever you need to get an overview of all UTXOs the user has.
   *
   * @returns An array of Utxo objects.
   *
   */
  getAllUtxos(): Utxo[] {
    var allUtxos: Utxo[] = [];

    for (const tokenBalance of this.balance.tokenBalances.values()) {
      allUtxos.push(...tokenBalance.utxos.values());
    }
    return allUtxos;
  }

  // TODO: do checks based on IDL, are all accounts set, are all amounts which are not applicable zero?

  /**
   * Stores a data message as a UTXO (unspent transaction output) in the user's account.
   * - If the `shield` flag is set to true, it creates a shield transaction parameter using the data message.
   * - If the `shield` flag is set to false, it attempts to find a UTXO with a non-zero SOL balance and creates a transfer transaction parameter using the data message.
   * - The transaction parameters are then used to carry out the transaction.
   *
   * @param message - A Buffer object containing the data to be stored.
   * @param shield - A boolean flag indicating whether to use the shield action for storing the data.
   *
   * @throws `UserError`
   * - If the size of the message exceeds the maximum allowed message size.
   * - If no UTXO with sufficient SOL balance is found.
   *
   * @returns The result of the transaction.
   *
   */
  async storeData(message: Buffer, shield: boolean = false) {
    if (message.length > MAX_MESSAGE_SIZE)
      throw new UserError(
        UserErrorCode.MAX_STORAGE_MESSAGE_SIZE_EXCEEDED,
        "storeData",
        `${message.length}/${MAX_MESSAGE_SIZE}`,
      );
    if (shield) {
      await this.createShieldTransactionParameters({
        token: "SOL",
        publicAmountSol: new BN(0),
        minimumLamports: false,
        message,
        verifierIdl: IDL_VERIFIER_PROGRAM_STORAGE,
      });
    } else {
      var inUtxos: Utxo[] = [];
      // any utxo just select any utxo with a non-zero sol balance preferably sol balance
      const firstSolUtxo = this.balance.tokenBalances
        .get(SystemProgram.programId.toBase58())
        ?.utxos.values()
        .next().value;
      if (firstSolUtxo) {
        inUtxos.push(firstSolUtxo);
      } else {
        // take the utxo with the biggest sol balance
        // 1. get all utxos
        // 2. sort descending
        // 3. select biggest which is in index 0
        var allUtxos = this.getAllUtxos();
        allUtxos.sort((a, b) => a.amounts[0].sub(b.amounts[0]).toNumber());
        inUtxos.push(allUtxos[0]);
      }
      if (inUtxos.length === 0 || inUtxos[0] === undefined)
        throw new UserError(
          SelectInUtxosErrorCode.FAILED_TO_SELECT_SOL_UTXO,
          "storeData",
        );

      const tokenCtx = TOKEN_REGISTRY.get("SOL")!;

      const txParams = await TransactionParameters.getTxParams({
        tokenCtx,
        action: Action.TRANSFER,
        account: this.account,
        inUtxos,
        provider: this.provider,
        relayer: this.provider.relayer,
        transactionNonce: this.balance.transactionNonce,
        appUtxo: this.appUtxoConfig,
        message,
        mergeUtxos: true,
        addInUtxos: false,
        verifierIdl: IDL_VERIFIER_PROGRAM_STORAGE,
        assetLookupTable: this.provider.lookUpTables.assetLookupTable,
        verifierProgramLookupTable:
          this.provider.lookUpTables.verifierProgramLookupTable,
      });
      this.recentTransactionParameters = txParams;
    }
    return this.transactWithParameters({
      txParams: this.recentTransactionParameters!,
    });
  }

  async executeAppUtxo({
    appUtxo,
    outUtxos,
    action,
    programParameters,
  }: {
    appUtxo: Utxo;
    outUtxos?: Utxo[];
    action: Action;
    programParameters: any;
    recipient?: Account;
  }) {
    if (!programParameters.verifierIdl)
      throw new UserError(
        UtxoErrorCode.APP_DATA_IDL_UNDEFINED,
        "executeAppUtxo",
        `provided program parameters: ${programParameters}`,
      );
    if (action === Action.TRANSFER) {
      let txParams = await this.createTransferTransactionParameters({
        verifierIdl: IDL_VERIFIER_PROGRAM_TWO,
        inUtxos: [appUtxo],
        outUtxos,
        addInUtxos: false,
        addOutUtxos: outUtxos ? false : true,
      });
      return this.transactWithParameters({
        txParams,
        appParams: programParameters,
      });
    } else {
      throw new Error("Not implemented");
    }
  }

  async getProgramUtxos({
    latestBalance = true,
    latestInboxBalance = true,
    idl,
    asMap = false,
  }: {
    latestBalance?: boolean;
    latestInboxBalance?: boolean;
    idl: Idl;
    aes?: boolean;
    asMap?: boolean;
  }) {
    const programAddress = TransactionParameters.getVerifierProgramId(idl);
    const balance = latestBalance
      ? await this.syncStorage(idl, true)
      : this.balance.programBalances;
    const inboxBalance = latestInboxBalance
      ? await this.syncStorage(idl, false)
      : this.inboxBalance.programBalances;

    const programBalance = balance?.get(programAddress.toBase58());
    const inboxProgramBalance = inboxBalance?.get(programAddress.toBase58());

    if (asMap)
      return {
        tokenBalances: programBalance?.tokenBalances,
        inboxTokenBalances: inboxProgramBalance?.tokenBalances,
      };
    var programUtxoArray: Utxo[] = [];
    if (programBalance) {
      for (var tokenBalance of programBalance.tokenBalances.values()) {
        programUtxoArray.push(...tokenBalance.utxos.values());
      }
    }
    var inboxProgramUtxoArray: Utxo[] = [];
    if (inboxProgramBalance) {
      for (var tokenBalance of inboxProgramBalance.tokenBalances.values()) {
        inboxProgramUtxoArray.push(...tokenBalance.utxos.values());
      }
    }
    return { programUtxoArray, inboxProgramUtxoArray };
  }

  async getUtxo(
    commitment: string,
    latest: boolean = false,
    idl?: Idl,
  ): Promise<{ utxo: Utxo; status: string } | undefined> {
    if (latest) {
      await this.getBalance();
      if (idl) {
        await this.syncStorage(idl, true);
        await this.syncStorage(idl, false);
      }
    }

    const iterateOverTokenBalance = (
      tokenBalances: Map<string, TokenUtxoBalance>,
    ) => {
      for (var [token, tokenBalance] of tokenBalances) {
        const utxo = tokenBalance.utxos.get(commitment);
        if (utxo) {
          return { status: "ready", utxo };
        }
        const spentUtxo = tokenBalance.spentUtxos.get(commitment);
        if (spentUtxo) {
          return { status: "spent", utxo: spentUtxo };
        }
        const committedUtxo = tokenBalance.committedUtxos.get(commitment);
        if (committedUtxo) {
          return { status: "committed", utxo: committedUtxo };
        }
      }
    };
    let res = undefined;
    for (var [program, programBalance] of this.balance.programBalances) {
      res = iterateOverTokenBalance(programBalance.tokenBalances);
      if (res) return res;
    }
    res = iterateOverTokenBalance(this.balance.tokenBalances);
    return res;
  }
}
