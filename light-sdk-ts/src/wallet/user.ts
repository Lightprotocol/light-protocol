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
  getAccountUtxos,
  SolMerkleTree,
  SIGN_MESSAGE,
  AUTHORITY,
  MERKLE_TREE_KEY,
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
} from "../index";

const message = new TextEncoder().encode(SIGN_MESSAGE);

export type Balance = {
  symbol: string;
  amount: BN;
  tokenAccount: PublicKey;
  decimals: BN;
};

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
// TODO: add "balances, pending Balances, incoming utxos " / or find a better alternative
export class User {
  provider: Provider;
  account?: Account;
  utxos?: Utxo[];
  spentUtxos?: Utxo[];
  private seed?: string;
  transactionIndex: number;

  constructor({
    provider,
    utxos = [],
    spentUtxos = [],
    account = undefined,
    transactionIndex,
  }: {
    provider: Provider;
    utxos?: Utxo[];
    spentUtxos?: Utxo[];
    account?: Account;
    transactionIndex?: number;
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
    this.utxos = utxos;
    this.spentUtxos = spentUtxos;
    this.account = account;
    this.transactionIndex = transactionIndex ? transactionIndex : 0;
  }

  async getUtxos(
    aes: boolean = true,
  ): Promise<{ decryptedUtxos: Utxo[]; spentUtxos: Utxo[] }> {
    let leavesPdas = await SolMerkleTree.getInsertedLeaves(
      MERKLE_TREE_KEY,
      this.provider.provider,
    );

    //TODO: add: "pending" to balances
    //TODO: add init by cached (subset of leavesPdas)
    const params = {
      leavesPdas,
      provider: this.provider.provider!,
      account: this.account!,
      poseidon: this.provider.poseidon,
      merkleTreeProgram: merkleTreeProgramId,
      merkleTree: this.provider.solMerkleTree!.merkleTree!,
      transactionIndex: 0,
      merkleTreePdaPublicKey: MERKLE_TREE_KEY,
      aes,
    };
    // does only aes encrypted change utxos, nacl encrypted utxos from
    const { decryptedUtxos, spentUtxos, transactionIndex } =
      await getAccountUtxos(params);

    this.transactionIndex = transactionIndex;
    this.utxos = decryptedUtxos;
    this.spentUtxos = spentUtxos;
    console.log("✔️ updated utxos", this.utxos.length);
    console.log("✔️ spent updated utxos", this.spentUtxos.length);
    return { decryptedUtxos, spentUtxos };
  }

  async getBalance({
    latest = true,
  }: {
    latest?: boolean;
  }): Promise<Balance[]> {
    const balances: Balance[] = [];
    if (!this.utxos)
      throw new UserError(
        UserErrorCode.UTXOS_NOT_INITIALIZED,
        "getBalances",
        "Utxos not initialized",
      );
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
      await this.getUtxos();
    } else {
      console.log("✔️ read utxos from cache", this.utxos.length);
    }

    // add permissioned tokens to balance display
    TOKEN_REGISTRY.forEach((token) => {
      balances.push({
        symbol: token.symbol,
        amount: new BN(0),
        tokenAccount: token.tokenAccount,
        decimals: token.decimals,
      });
    });

    this.utxos.forEach((utxo) => {
      utxo.assets.forEach((asset, i) => {
        const tokenAccount = asset;
        const amount = utxo.amounts[i];

        const existingBalance = balances.find(
          (balance) =>
            balance.tokenAccount.toBase58() === tokenAccount.toBase58(),
        );
        if (existingBalance) {
          existingBalance.amount = existingBalance.amount.add(amount);
        } else {
          let tokenData = TOKEN_REGISTRY.find(
            (t) => t.tokenAccount.toBase58() === tokenAccount.toBase58(),
          );
          if (!tokenData)
            throw new UserError(
              UserErrorCode.TOKEN_NOT_FOUND,
              "getBalance",
              `Token ${tokenAccount.toBase58()} not found in registry`,
            );
          balances.push({
            symbol: tokenData.symbol,
            amount,
            tokenAccount,
            decimals: tokenData.decimals,
          });
        }
      });
    });
    // TODO: add "pending" balances,
    return balances;
  }

  // TODO: in UI, support wallet switching, "prefill option with button"

  /**
   *
   * @param amount e.g. 1 SOL = 1, 2 USDC = 2
   * @param token "SOL", "USDC", "USDT",
   * @param recipient optional, if not set, will shield to self
   * @param extraSolAmount optional, if set, will add extra SOL to the shielded amount
   * @param senderTokenAccount optional, if set, will use this token account to shield from, else derives ATA
   */
  async shieldCreateTransactionParameters({
    token,
    publicAmountSpl,
    recipient,
    publicAmountSol,
    senderTokenAccount,
    minimumLamports = true,
    appUtxo,
  }: {
    token: string;
    recipient?: Account;
    publicAmountSpl?: number | BN | string;
    publicAmountSol?: number | BN | string;
    minimumLamports?: boolean;
    senderTokenAccount?: PublicKey;
    appUtxo?: AppUtxoConfig;
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
    // if (recipient)
    //   throw new Error("Shields to other users not implemented yet!");
    let tokenCtx = TOKEN_REGISTRY.find((t) => t.symbol === token);
    if (!tokenCtx)
      throw new UserError(
        UserErrorCode.TOKEN_NOT_FOUND,
        "shield",
        "Token not supported!",
      );
    if (tokenCtx.isSol && senderTokenAccount)
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

    // TODO: refactor this is ugly
    if (!tokenCtx.isSol && publicAmountSpl) {
      if (senderTokenAccount) {
        userSplAccount = senderTokenAccount;
      } else {
        userSplAccount = splToken.getAssociatedTokenAddressSync(
          tokenCtx!.tokenAccount,
          this.provider!.wallet!.publicKey,
        );
      }

      // let tokenBalance = await splToken.getAccount(
      //   this.provider.provider?.connection!,
      //   userSplAccount,
      // );

      // if (!tokenBalance)
      //   throw new UserError(
      //     UserErrorCode.ASSOCIATED_TOKEN_ACCOUNT_DOESNT_EXIST,
      //     "shield",
      //     "AssociatdTokenAccount doesn't exist!",
      //   );

      // if (publicAmountSpl.gte(new BN(tokenBalance.amount.toString())))
      //   throw new UserError(
      //     UserErrorCode.INSUFFICIENT_BAlANCE,
      //     "shield",
      //     `Insufficient token balance! ${publicAmountSpl.toString()} bal: ${tokenBalance!
      //       .amount!}`,
      //   );

      // try {
      //   const transaction = new SolanaTransaction().add(
      //     splToken.createApproveInstruction(
      //       userSplAccount,
      //       AUTHORITY,
      //       this.provider.wallet!.publicKey,
      //       publicAmountSpl.toNumber(),
      //       [this.provider.wallet!.publicKey],
      //     ),
      //   );

      //   await this.provider.wallet!.sendAndConfirmTransaction(transaction);
      // } catch (e) {
      //   throw new UserError(
      //     UserErrorCode.APPROVE_ERROR,
      //     "shield",
      //     `Error approving token transfer! ${e}`,
      //   );
      // }
    }
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
    }
    const txParams = await TransactionParameters.getTxParams({
      tokenCtx,
      action: Action.SHIELD,
      account: this.account,
      utxos: this.utxos,
      publicAmountSol,
      publicAmountSpl,
      userSplAccount,
      provider: this.provider,
      transactionIndex: this.transactionIndex,
      appUtxo,
      verifier: new VerifierZero(),
      outUtxos,
      addInUtxos: recipient ? false : true,
      addOutUtxos: recipient ? false : true,
    });
    this.recentTransactionParameters = txParams;
    return txParams;
    // return await this.transactWithParameters({ txParams });
  }
  recentTransactionParameters?: TransactionParameters;
  recentTransaction?: Transaction;
  approved?: boolean;

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
        // userSplAccount,
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
    this.transactionIndex += 1;

    // TODO: this needs to be revisited because other parts of the sdk still
    // assume that either every transaction just has one key
    // and that one transaction just contains one aes utxo
    // increases transactionIndex for every of my own utxos
    this.recentTransactionParameters?.outputUtxos.map((utxo) => {
      if (utxo.account.pubkey.toString() === this.account?.pubkey.toString()) {
        this.transactionIndex += 1;
      }
    });
    return txHash;
  }

  async updateMerkleTree() {
    const response = await this.provider.relayer.updateMerkleTree(
      this.provider,
    );

    if (this.recentTransactionParameters) {
      this.spentUtxos = getUpdatedSpentUtxos(
        this.recentTransactionParameters.inputUtxos,
        this.spentUtxos,
      );
    }
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
    recipient?: Account;
    publicAmountSpl?: number | BN | string;
    publicAmountSol?: number | BN | string;
    minimumLamports?: boolean;
    senderTokenAccount?: PublicKey;
    appUtxo?: AppUtxoConfig;
  }) {
    await this.shieldCreateTransactionParameters({
      token,
      publicAmountSpl,
      recipient,
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
    await this.unshieldCreateTransactionParameters({
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
  // TODO: add pass-in tokenAccount
  /**
   * @params token: string
   * @params amount: number - in base units (e.g. lamports for 'SOL')
   * @params recipient: PublicKey - Solana address
   * @params extraSolAmount: number - optional, if not set, will use MINIMUM_LAMPORTS
   */
  async unshieldCreateTransactionParameters({
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
    const tokenCtx = TOKEN_REGISTRY.find((t) => t.symbol === token);
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

    if (!tokenCtx.isSol && publicAmountSpl) {
      let tokenBalance = await this.provider.connection?.getTokenAccountBalance(
        recipientSpl,
      );
      if (!tokenBalance?.value.uiAmount) {
        /** Signal relayer to create the ATA and charge an extra fee for it */
        ataCreationFee = true;
      }
      recipientSpl = splToken.getAssociatedTokenAddressSync(
        tokenCtx!.tokenAccount,
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

    const txParams = await TransactionParameters.getTxParams({
      tokenCtx,
      publicAmountSpl: _publicSplAmount,
      action: Action.UNSHIELD,
      account: this.account,
      utxos: this.utxos,
      publicAmountSol: _publicSolAmount,
      recipientSol: recipientSol,
      recipientSplAddress: recipientSpl,
      provider: this.provider,
      relayer: this.provider.relayer,
      ataCreationFee,
      transactionIndex: this.transactionIndex,
      verifier: new VerifierZero(),
    });
    this.recentTransactionParameters = txParams;
    return txParams; //await this.transactWithParameters({ txParams });
  }
  async transfer({
    token,
    // alternatively we could use the recipient type here as well
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
    await this.transferCreateTransactionParameters({
      token,
      recipient,
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
  async transferCreateTransactionParameters({
    token,
    // alternatively we could use the recipient type here as well
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
    if (!recipient)
      throw new UserError(
        UserErrorCode.SHIELDED_RECIPIENT_UNDEFINED,
        "transfer",
        "No shielded recipient provided for transfer.",
      );

    if (!amountSol && !amountSpl)
      throw new UserError(
        UserErrorCode.NO_AMOUNTS_PROVIDED,
        "transfer",
        "Need to provide at least one amount for an unshield",
      );
    const tokenCtx = TOKEN_REGISTRY.find((t) => t.symbol === token);
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
          mint: tokenCtx.tokenAccount,
          account: recipient,
          solAmount: parsedSolAmount,
          splAmount: parsedSplAmount,
          appUtxo,
        },
      ],
      poseidon: this.provider.poseidon,
    });
    const txParams = await TransactionParameters.getTxParams({
      tokenCtx,
      action: Action.TRANSFER,
      account: this.account,
      utxos: this.utxos,
      outUtxos,
      provider: this.provider,
      relayer: this.provider.relayer,
      transactionIndex: this.transactionIndex,
      verifier: new VerifierZero(),
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
    let tx = new Transaction({
      provider: this.provider,
      params: txParams,
      appParams,
    });

    await tx.compileAndProve();

    let txHash;
    try {
      txHash = await tx.sendAndConfirmTransaction();
    } catch (e) {
      throw new UserError(
        TransactionErrorCode.SEND_TRANSACTION_FAILED,
        "shield",
        `Error in tx.sendAndConfirmTransaction! ${e}`,
      );
    }

    // TODO: this needs to be revisited because other parts of the sdk still
    // assume that either every transaction just has one key
    // and that one transaction just contains one aes utxo
    // increases transactionIndex for every of my own utxos
    txParams.outputUtxos.map((utxo) => {
      if (utxo.account.pubkey.toString() === this.account?.pubkey.toString()) {
        this.transactionIndex += 1;
      }
    });

    const response = await this.provider.relayer.updateMerkleTree(
      this.provider,
    );

    this.spentUtxos = getUpdatedSpentUtxos(
      txParams.inputUtxos,
      this.spentUtxos,
    );

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
  }) {}

  // TODO: consider removing payer property completely -> let user pass in the payer for 'load' and for 'shield' only.
  // TODO: evaluate whether we could use an offline instance of user, for example to generate a proof offline, also could use this to move error test to sdk
  /**
   *
   * @param cachedUser - optional cached user object
   * untested for browser wallet!
   */

  async init(provider: Provider, seed?: string, utxos?: Utxo[]) {
    if (seed) {
      this.seed = seed;
      this.utxos = utxos; // TODO: potentially add encr/decryption
      if (!provider)
        throw new UserError(
          ProviderErrorCode.PROVIDER_UNDEFINED,
          "load",
          "No provider provided",
        );
      this.provider = provider;
    }
    if (!this.seed) {
      if (this.provider.wallet) {
        const signature: Uint8Array = await this.provider.wallet.signMessage(
          message,
        );
        this.seed = new anchor.BN(signature).toString();
      } else {
        throw new UserError(
          UserErrorCode.NO_WALLET_PROVIDED,
          "load",
          "No payer or browser wallet provided",
        );
      }
    }

    // get the provider?
    if (!this.provider.poseidon) {
      this.provider.poseidon = await circomlibjs.buildPoseidonOpt();
    }
    if (!this.account) {
      this.account = new Account({
        poseidon: this.provider.poseidon,
        seed: this.seed,
      });
    }

    // TODO: optimize: fetch once, then filter
    await this.getUtxos();
  }

  // TODO: we need a non-anchor version of "provider" - (bundle functionality exposed by the wallet adapter into own provider-like class)
  /**
   *
   * @param provider - Light provider
   * @param seed - Optional user seed to instantiate from; e.g. if the seed is supplied, skips the log-in signature prompt.
   * @param utxos - Optional user utxos to instantiate from
   */

  static async init(
    provider: Provider,
    seed?: string,
    utxos?: Utxo[],
  ): Promise<any> {
    try {
      const user = new User({ provider });
      await user.init(provider, seed, utxos);
      return user;
    } catch (e) {
      throw new UserError(
        UserErrorCode.LOAD_ERROR,
        "load",
        `Error while loading user! ${e}`,
      );
    }
  }

  // TODO: find clean way to support this (accepting/rejecting utxos, checking "available balance"),...
  /** shielded transfer to self, merge 10-1; per asset (max: 5-1;5-1)
   * check *after* ACTION whether we can still merge in more.
   * TODO: add dust tagging protection (skip dust utxos)
   * Still torn - for regular devs this should be done automatically, e.g auto-prefacing any regular transaction.
   * whereas for those who want manual access there should be a fn to merge -> utxo = getutxosstatus() -> merge(utxos)
   */
  async mergeUtxos() {
    throw new Error("not implemented yet");
  }

  async getLatestTransactionHistory() {
    throw new Error("not implemented yet");
  }
  // TODO: add proof-of-origin call.
  // TODO: merge with getUtxoStatus?
  // returns all non-accepted utxos.
  // we'd like to enforce some kind of sanitary controls here.
  // would not be part of the main balance
  getUtxoInbox() {
    throw new Error("not implemented yet");
  }
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
