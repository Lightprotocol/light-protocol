import {
  PublicKey,
  SystemProgram,
  Transaction as SolanaTransaction,
  TransactionConfirmationStrategy,
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
export enum ConfirmOptions {
  finalized = "finalized",
  spendable = "spendable",
}

/**
 *
 * @param provider Either a nodeProvider or browserProvider
 * @param account User account (optional)
 * @param utxos User utxos (optional)
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

  constructor({
    provider,
    serializedUtxos, // balance
    serialiezdSpentUtxos, // inboxBalance idk
    account,
    appUtxoConfig,
    verifierIdl = IDL_VERIFIER_PROGRAM_ZERO,
  }: {
    provider: Provider;
    serializedUtxos?: Buffer;
    serialiezdSpentUtxos?: Buffer;
    account: Account;
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
      totalSolBalance: new BN(0),
    };
    this.inboxBalance = {
      tokenBalances: new Map([
        [SystemProgram.programId.toBase58(), TokenUtxoBalance.initSol()],
      ]),
      programBalances: new Map(),
      nftBalances: new Map(),
      numberInboxUtxos: 0,
      totalSolBalance: new BN(0),
    };
  }

  // TODO: should update merkle tree as well
  // TODO: test robustness
  // TODO: nonce incrementing is very ugly revisit
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
          // tokenBalance.utxos.delete(key)
          tokenBalance.moveToSpentUtxos(key);
        }
      }
    }

    if (!this.provider)
      throw new UserError(ProviderErrorCode.PROVIDER_UNDEFINED, "syncState");
    if (!this.provider.provider)
      throw new UserError(UserErrorCode.PROVIDER_NOT_INITIALIZED, "syncState");
    // TODO: adapt to indexedTransactions such that this works with verifier two for

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
        await decryptAddUtxoToBalance({
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
          verifierProgramLookupTable:
            this.provider.lookUpTables.verifierProgramLookupTable,
          assetLookupTable: this.provider.lookUpTables.assetLookupTable,
        });
        await decryptAddUtxoToBalance({
          encBytes: Buffer.from(
            trx.encryptedUtxos.slice(
              120,
              120 + NACL_ENCRYPTED_COMPRESSED_UTXO_BYTES_LENGTH,
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
          verifierProgramLookupTable:
            this.provider.lookUpTables.verifierProgramLookupTable,
          assetLookupTable: this.provider.lookUpTables.assetLookupTable,
        });
      }
    }

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
   * returns all non-accepted utxos.
   * would not be part of the main balance
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
   *
   * @param amount e.g. 1 SOL = 1, 2 USDC = 2
   * @param token "SOL", "USDC", "USDT",
   * @param recipient optional, if not set, will shield to self
   * @param extraSolAmount optional, if set, will add extra SOL to the shielded amount
   * @param senderTokenAccount optional, if set, will use this token account to shield from, else derives ATA
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
        transaction.recentBlockhash = (
          await this.provider.provider?.connection.getLatestBlockhash(
            "confirmed",
          )
        )?.blockhash;
        await this.provider.wallet!.sendAndConfirmTransaction(transaction);
        this.approved = true;
      } catch (e) {
        throw new UserError(
          UserErrorCode.APPROVE_ERROR,
          "shield",
          `Error approving token transfer! ${e.stack}`,
        );
      }
    } else {
      this.approved = true;
    }
  }

  async sendTransaction(
    confirmOptions: ConfirmOptions = ConfirmOptions.spendable,
  ) {
    if (!this.recentTransactionParameters)
      throw new UserError(
        UserErrorCode.TRANSACTION_PARAMTERS_UNDEFINED,
        "sendTransaction",
        "transaction needs to be compiled and a proof generated before send.",
      );
    if (
      this.recentTransactionParameters?.action === Action.SHIELD &&
      !this.approved
    )
      throw new UserError(
        UserErrorCode.SPL_FUNDS_NOT_APPROVED,
        "sendTransaction",
        "spl funds need to be approved before a shield with spl tokens can be executed",
      );
    if (!this.recentTransaction)
      throw new UserError(
        UserErrorCode.TRANSACTION_UNDEFINED,
        "sendTransaction",
        "transaction needs to be compiled and a proof generated before send.",
      );
    let txResult;
    try {
      txResult = await this.recentTransaction.sendTransaction(confirmOptions);
    } catch (e) {
      throw new UserError(
        TransactionErrorCode.SEND_TRANSACTION_FAILED,
        "shield",
        `Error in tx.sendTransaction ${e}`,
      );
    }
    let transactionContainsEncryptedUtxo = false;
    this.recentTransactionParameters.outputUtxos.map((utxo) => {
      if (utxo.account.pubkey.toString() === this.account?.pubkey.toString()) {
        transactionContainsEncryptedUtxo = true;
      }
    });
    return txResult;
  }

  resetTxState() {
    this.recentTransaction = undefined;
    this.recentTransactionParameters = undefined;
    this.approved = undefined;
  }

  /**
   *
   * @param amount e.g. 1 SOL = 1, 2 USDC = 2
   * @param token "SOL", "USDC", "USDT",
   * @param recipient optional, if not set, will shield to self
   * @param extraSolAmount optional, if set, will add extra SOL to the shielded amount
   * @param senderTokenAccount optional, if set, will use this token account to shield from, else derives ATA
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
    confirmOptions = ConfirmOptions.spendable,
  }: {
    token: string;
    recipient?: string;
    publicAmountSpl?: number | BN | string;
    publicAmountSol?: number | BN | string;
    minimumLamports?: boolean;
    senderTokenAccount?: PublicKey;
    appUtxo?: AppUtxoConfig;
    skipDecimalConversions?: boolean;
    confirmOptions?: ConfirmOptions;
  }) {
    let recipientAccount = recipient
      ? Account.fromPubkey(recipient, this.provider.poseidon)
      : undefined;

    const txParams = await this.createShieldTransactionParameters({
      token,
      publicAmountSpl,
      recipient: recipientAccount,
      publicAmountSol,
      senderTokenAccount,
      minimumLamports,
      appUtxo,
      skipDecimalConversions,
    });
    return await this.transactWithParameters({ txParams, confirmOptions });
  }

  async unshield({
    token,
    publicAmountSpl,
    publicAmountSol,
    recipient = AUTHORITY,
    minimumLamports = true,
    confirmOptions = ConfirmOptions.spendable,
  }: {
    token: string;
    recipient?: PublicKey;
    publicAmountSpl?: number | BN | string;
    publicAmountSol?: number | BN | string;
    minimumLamports?: boolean;
    confirmOptions?: ConfirmOptions;
  }) {
    const txParams = await this.createUnshieldTransactionParameters({
      token,
      publicAmountSpl,
      publicAmountSol,
      recipient,
      minimumLamports,
    });
    return await this.transactWithParameters({ txParams, confirmOptions });
  }

  // TODO: add unshieldSol and unshieldSpl
  // TODO: add optional passs-in token mint
  // TODO: add pass-in mint
  /**
   * @params token: string
   * @params amount: number - in base units (e.g. lamports for 'SOL')
   * @params recipient: PublicKey - Solana address
   * @params extraSolAmount: number - optional, if not set, will use MINIMUM_LAMPORTS
   */
  async createUnshieldTransactionParameters({
    token,
    publicAmountSpl,
    publicAmountSol,
    recipient = AUTHORITY,
    minimumLamports = true,
  }: {
    token: string;
    recipient?: PublicKey;
    publicAmountSpl?: number | BN | string;
    publicAmountSol?: number | BN | string;
    minimumLamports?: boolean;
  }) {
    const tokenCtx = TOKEN_REGISTRY.get(token);
    if (!tokenCtx)
      throw new UserError(
        UserErrorCode.TOKEN_NOT_FOUND,
        "unshield",
        "Token not supported!",
      );

    if (!publicAmountSpl && !publicAmountSol)
      throw new UserError(
        CreateUtxoErrorCode.NO_PUBLIC_AMOUNTS_PROVIDED,
        "unshield",
        "Need to provide at least one amount for an unshield",
      );
    if (publicAmountSol && recipient.toBase58() == AUTHORITY.toBase58())
      throw new UserError(
        TransactionErrorCode.SOL_RECIPIENT_UNDEFINED,
        "getTxParams",
        "no recipient provided for sol unshield",
      );
    if (publicAmountSpl && recipient.toBase58() == AUTHORITY.toBase58())
      throw new UserError(
        TransactionErrorCode.SPL_RECIPIENT_UNDEFINED,
        "getTxParams",
        "no recipient provided for spl unshield",
      );

    let ataCreationFee = false;
    let recipientSpl = undefined;
    if (publicAmountSpl) {
      let tokenBalance = await this.provider.connection?.getTokenAccountBalance(
        recipient,
      );
      if (!tokenBalance?.value.uiAmount) {
        /** Signal relayer to create the ATA and charge an extra fee for it */
        ataCreationFee = true;
      }
      recipientSpl = splToken.getAssociatedTokenAddressSync(
        tokenCtx!.mint,
        recipient,
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
      recipientSol: recipient,
      recipientSplAddress: recipientSpl,
      provider: this.provider,
      relayer: this.provider.relayer,
      ataCreationFee,
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
  async transfer({
    token,
    recipient,
    amountSpl,
    amountSol,
    appUtxo,
    confirmOptions = ConfirmOptions.spendable,
  }: {
    token: string;
    amountSpl?: BN | number | string;
    amountSol?: BN | number | string;
    recipient: string;
    appUtxo?: AppUtxoConfig;
    confirmOptions?: ConfirmOptions;
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
    return this.transactWithParameters({ txParams, confirmOptions });
  }

  // TODO: add separate lookup function for users.
  // TODO: add account parsing from and to string which is concat shielded pubkey and encryption key
  /**
   * @description transfers to one recipient utxo and creates a change utxo with remainders of the input
   * @param token mint
   * @param amount
   * @param recipient shieldedAddress (BN)
   * @param recipientEncryptionPublicKey (use strToArr)
   * @returns
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

  async transactWithParameters({
    txParams,
    appParams,
    confirmOptions,
  }: {
    txParams: TransactionParameters;
    appParams?: any;
    confirmOptions?: ConfirmOptions;
  }) {
    this.recentTransactionParameters = txParams;

    const tx = await this.compileAndProveTransaction(appParams);
    //console.log('tx: ',tx)
    await this.approve();
    this.approved = true;

    // we send an array of instructions to the relayer and the relayer sends 3 transaction
    const txHash = await this.sendTransaction(confirmOptions);

    var relayerMerkleTreeUpdateResponse = "notPinged";

    if (confirmOptions === ConfirmOptions.finalized) {
      this.provider.relayer.updateMerkleTree(this.provider);
      relayerMerkleTreeUpdateResponse = "pinged relayer";
    }

    if (confirmOptions === ConfirmOptions.spendable) {
      await this.provider.relayer.updateMerkleTree(this.provider);
      relayerMerkleTreeUpdateResponse = "success";
    }

    await this.getBalance();

    this.resetTxState();
    return { txHash, response: relayerMerkleTreeUpdateResponse };
  }

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
   *
   * @param provider - Light provider
   * @param seed - Optional user seed to instantiate from; e.g. if the seed is supplied, skips the log-in signature prompt.
   * @param utxos - Optional user utxos to instantiate from
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
  /** shielded transfer to self, merge 10-1;
   * get utxo inbox
   * merge highest first
   * loops in steps of 9 or 10
   */
  async mergeAllUtxos(
    asset: PublicKey,
    confirmOptions: ConfirmOptions = ConfirmOptions.spendable,
    latest: boolean = true,
  ) {
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
    return await this.transactWithParameters({ txParams, confirmOptions });
  }

  // TODO: how do we handle app utxos?, some will not be able to be accepted we can only mark these as accepted
  /** shielded transfer to self, merge 10-1;
   * get utxo inbox
   * merge highest first
   * loops in steps of 9 or 10
   */
  async mergeUtxos(
    commitments: string[],
    asset: PublicKey,
    confirmOptions: ConfirmOptions = ConfirmOptions.spendable,
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
    return this.transactWithParameters({ txParams, confirmOptions });
  }

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

  getUtxoStatus() {
    throw new Error("not implemented yet");
  }
  // getPrivacyScore() -> for unshields only, can separate into its own helper method
  // Fetch utxos should probably be a function such the user object is not occupied while fetching
  // but it would probably be more logical to fetch utxos here as well
  addUtxos() {
    throw new Error("not implemented yet");
  }

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

  /**
   * is shield or transfer
   */
  // TODO: group shield parameters into type
  // TODO: group transfer parameters into type
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
    confirmOptions = ConfirmOptions.spendable,
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
    confirmOptions?: ConfirmOptions;
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

    return this.transactWithParameters({ txParams, confirmOptions });
  }

  // TODO: add storage transaction nonce to rotate keypairs
  /**
   * - get indexed transactions for a storage compressed account
   * - try to decrypt all and add to appUtxos or decrypted data map
   * - add custom descryption strategies for arbitrary data
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
            tokenBalance.moveToSpentUtxos(key);
          }
        }
      }
    }
    return this.balance.programBalances;
  }

  getAllUtxos(): Utxo[] {
    var allUtxos: Utxo[] = [];

    for (const tokenBalance of this.balance.tokenBalances.values()) {
      allUtxos.push(...tokenBalance.utxos.values());
    }
    return allUtxos;
  }

  // TODO: do checks based on IDL, are all accounts set, are all amounts which are not applicable zero?
  /**
   *
   */
  async storeData(
    message: Buffer,
    confirmOptions: ConfirmOptions = ConfirmOptions.spendable,
    shield: boolean = false,
  ) {
    if (message.length > MAX_MESSAGE_SIZE)
      throw new UserError(
        UserErrorCode.MAX_STORAGE_MESSAGE_SIZE_EXCEEDED,
        "storeData",
        `${message.length}/${MAX_MESSAGE_SIZE}`,
      );
    if (shield) {
      const txParams = await this.createShieldTransactionParameters({
        token: "SOL",
        publicAmountSol: new BN(0),
        minimumLamports: false,
        message,
        verifierIdl: IDL_VERIFIER_PROGRAM_STORAGE,
      });
      this.recentTransactionParameters = txParams;
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
      confirmOptions,
    });
  }

  async executeAppUtxo({
    appUtxo,
    outUtxos,
    action,
    programParameters,
    confirmOptions,
  }: {
    appUtxo: Utxo;
    outUtxos?: Utxo[];
    action: Action;
    programParameters: any;
    recipient?: Account;
    confirmOptions?: ConfirmOptions;
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
        confirmOptions,
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
