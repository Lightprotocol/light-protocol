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
  TRANSACTION_MERKLE_TREE_KEY,
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
  VerifierZero,
  createRecipientUtxos,
  VerifierTwo,
  Verifier,
  VerifierError,
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
} from "../index";
import { Idl } from "@coral-xyz/anchor";
const message = new TextEncoder().encode(SIGN_MESSAGE);

// TODO: Utxos should be assigned to a merkle tree
// TODO: evaluate optimization to store keypairs separately or store utxos in a map<Keypair, Utxo> to not store Keypairs repeatedly
// TODO: add support for wallet adapter (no access to payer keypair)

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
  verifier: Verifier;
  balance: Balance;
  inboxBalance: InboxBalance;

  constructor({
    provider,
    serializedUtxos, // balance
    serialiezdSpentUtxos, // inboxBalance idk
    account,
    transactionNonce,
    verifier = new VerifierZero(),
    appUtxoConfig,
    verifierIdl = IDL_VERIFIER_PROGRAM_ZERO,
  }: {
    provider: Provider;
    serializedUtxos?: Buffer;
    serialiezdSpentUtxos?: Buffer;
    account: Account;
    transactionNonce?: number;
    verifier?: Verifier;
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
    this.verifier = verifier;
    if (appUtxoConfig && !verifier.config.isAppVerifier)
      throw new UserError(
        UserErrorCode.VERIFIER_IS_NOT_APP_ENABLED,
        "constructor",
        `appUtxo config provided as default verifier but no app enabled verifier defined`,
      );
    this.appUtxoConfig = appUtxoConfig;
    this.balance = {
      tokenBalances: new Map([
        [SystemProgram.programId.toBase58(), TokenUtxoBalance.initSol()],
      ]),
      transactionNonce: 0,
      committedTransactionNonce: 0,
      decryptionTransactionNonce: 0,
      totalSolBalance: new BN(0),
    };
    this.inboxBalance = {
      tokenBalances: new Map([
        [SystemProgram.programId.toBase58(), TokenUtxoBalance.initSol()],
      ]),
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
  async syncState(
    aes: boolean = true,
    balance: Balance | InboxBalance,
    merkleTreePdaPublicKey: PublicKey,
  ): Promise<Balance | InboxBalance> {
    // reduce balance by spent utxos
    if (!this.provider.provider)
      throw new UserError(
        UserErrorCode.PROVIDER_NOT_INITIALIZED,
        "syncStat",
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
  }: {
    token: string;
    recipient?: Account;
    publicAmountSpl?: number | BN | string;
    publicAmountSol?: number | BN | string;
    minimumLamports?: boolean;
    senderTokenAccount?: PublicKey;
    appUtxo?: AppUtxoConfig;
    mergeExistingUtxos?: boolean;
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
    publicAmountSpl = publicAmountSpl
      ? convertAndComputeDecimals(publicAmountSpl, tokenCtx.decimals)
      : undefined;

    // if no sol amount by default min amount if disabled 0
    publicAmountSol = publicAmountSol
      ? convertAndComputeDecimals(publicAmountSol, new BN(1e9))
      : minimumLamports
      ? this.provider.minimumLamports
      : new BN(0);

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
      outUtxos.push(
        new Utxo({
          poseidon: this.provider.poseidon,
          assets: [SystemProgram.programId],
          amounts: [publicAmountSol],
          account: recipient,
          appDataHash: appUtxo?.appDataHash,
          verifierAddress: appUtxo?.verifierAddress,
          includeAppData: appUtxo?.includeAppData,
          appData: appUtxo?.appData,
        }),
      );
      // no merging of utxos when shielding to another recipient
      mergeExistingUtxos = false;
      utxos = [];
    }

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
      verifier: this.verifier,
      outUtxos,
      addInUtxos: recipient ? false : true,
      addOutUtxos: recipient ? false : true,
      verifierIdl: IDL_VERIFIER_PROGRAM_ZERO,
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
        this.recentTransactionParameters?.publicAmountSpl.gte(
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

  async updateMerkleTree() {
    const response = await this.provider.relayer.updateMerkleTree(
      this.provider,
    );

    // TODO: update
    // if (this.recentTransactionParameters) {
    //   this.spentUtxos = getUpdatedSpentUtxos(
    //     this.recentTransactionParameters.inputUtxos,
    //     this.spentUtxos,
    //   );
    // }

    this.recentTransaction = undefined;
    this.recentTransactionParameters = undefined;
    this.approved = undefined;
    return response;
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
  }: {
    token: string;
    recipient?: string;
    publicAmountSpl?: number | BN | string;
    publicAmountSol?: number | BN | string;
    minimumLamports?: boolean;
    senderTokenAccount?: PublicKey;
    appUtxo?: AppUtxoConfig;
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
    });
    await this.compileAndProveTransaction();
    await this.approve();
    const txHash = await this.sendAndConfirm();
    const response = await this.updateMerkleTree();
    return { txHash, response };
  }

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
   * @params token: string
   * @params amount: number - in base units (e.g. lamports for 'SOL')
   * @params recipient: PublicKey - Solana address
   * @params extraSolAmount: number - optional, if not set, will use MINIMUM_LAMPORTS
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
      verifier: this.verifier,
      appUtxo: this.appUtxoConfig,
      verifierIdl: IDL_VERIFIER_PROGRAM_ZERO,
    });
    this.recentTransactionParameters = txParams;
    return txParams; //await this.transactWithParameters({ txParams });
  }

  // TODO: replace recipient with recipient light publickey
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

    await this.createTransferTransactionParameters({
      token,
      recipient: recipientAccount,
      amountSpl,
      amountSol,
      appUtxo,
    });

    await this.compileAndProveTransaction();
    const txHash = await this.sendAndConfirm();
    const response = await this.updateMerkleTree();
    return { txHash, response };
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
  }: {
    token: string;
    amountSpl?: BN | number | string;
    amountSol?: BN | number | string;
    recipient: Account;
    appUtxo?: AppUtxoConfig;
  }) {
    if (!amountSol && !amountSpl)
      throw new UserError(
        UserErrorCode.NO_AMOUNTS_PROVIDED,
        "transfer",
        "Need to provide at least one amount for an unshield",
      );
    const tokenCtx = TOKEN_REGISTRY.get(token);
    if (!tokenCtx)
      throw new UserError(
        UserErrorCode.TOKEN_NOT_FOUND,
        "transfer",
        "Token not supported!",
      );

    var parsedSplAmount: BN = amountSpl
      ? convertAndComputeDecimals(amountSpl, tokenCtx.decimals)
      : new BN(0);
    // if no sol amount by default min amount if disabled 0
    const parsedSolAmount = amountSol
      ? convertAndComputeDecimals(amountSol, new BN(1e9))
      : new BN(0);

    let outUtxos = createRecipientUtxos({
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
    });
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
      action: Action.TRANSFER,
      account: this.account,
      utxos,
      outUtxos,
      provider: this.provider,
      relayer: this.provider.relayer,
      transactionNonce: this.balance.transactionNonce,
      verifier: this.verifier,
      appUtxo: this.appUtxoConfig,
      verifierIdl: IDL_VERIFIER_PROGRAM_ZERO,
    });
    this.recentTransactionParameters = txParams;
    return txParams; //await this.transactWithParameters({ txParams });
  }

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
      let verifier: Verifier = new VerifierZero();
      if (appUtxoConfig) {
        verifier = new VerifierTwo();
      }

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
      const user = new User({ provider, verifier, appUtxoConfig, account });

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

  /** shielded transfer to self, merge 10-1; per asset (max: 5-1;5-1)
   * check *after* ACTION whether we can still merge in more.
   * TODO: add dust tagging protection (skip dust utxos)
   * Still torn - for regular devs this should be done automatically, e.g auto-prefacing any regular transaction.
   * whereas for those who want manual access there should be a fn to merge -> utxo = getutxosstatus() -> merge(utxos)
   */
  async mergeUtxos() {
    throw new Error("not implemented yet");
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
}
