/// TODO: remove user class
import {
  LAMPORTS_PER_SOL,
  PublicKey,
  SystemProgram,
  Transaction as SolanaTransaction,
  TransactionInstruction,
} from "@solana/web3.js";

import { BN, Idl } from "@coral-xyz/anchor";
import * as splToken from "@solana/spl-token";

import { bs58 } from "@coral-xyz/anchor/dist/cjs/utils/bytes";
import {
  Balance,
  InboxBalance,
  TokenUtxoBalance,
  decryptAddUtxoToBalance,
  ProgramUtxoBalance,
} from "../build-balance";
import {
  BN_0,
  NACL_ENCRYPTED_COMPRESSED_UTXO_BYTES_LENGTH,
  UTXO_PREFIX_LENGTH,
  TOKEN_REGISTRY,
  AUTHORITY,
  TOKEN_PUBKEY_SYMBOL,
  MAX_MESSAGE_SIZE,
  MERKLE_TREE_SET,
} from "../constants";
import {
  UserError,
  UserErrorCode,
  ProviderErrorCode,
  RpcErrorCode,
  CreateUtxoErrorCode,
  TransactionParametersError,
  TransactionParametersErrorCode,
  TransactionErrorCode,
  UtxoError,
  AccountErrorCode,
  SelectInUtxosErrorCode,
} from "../errors";

import {
  IDL_LIGHT_PSP2IN2OUT,
  IDL_LIGHT_PSP10IN2OUT,
  IDL_LIGHT_PSP2IN2OUT_STORAGE,
} from "../idls";
import { MerkleTreeConfig } from "../merkle-tree";
import {
  getUserIndexTransactions,
  getTxParams,
  syncInputUtxosMerkleProofs,
  createSystemProofInputs,
  getSystemProof,
  getSystemPspIdl,
  getSolanaRemainingAccounts,
  prepareAccounts,
  createSolanaInstructions,
  getVerifierProgramId,
  Transaction,
} from "../transaction";
import {
  UserIndexedTransaction,
  AppUtxoConfig,
  Action,
  ActionResponseMulti,
  RpcIndexedTransactionResponse,
  Result,
} from "../types";
import {
  isProgramVerifier,
  fetchNullifierAccountInfo,
  decimalConversion,
  convertAndComputeDecimals,
} from "../utils";
import {
  OutUtxo,
  Utxo,
  createOutUtxo,
  createRecipientUtxos,
  ProgramUtxo,
  decryptProgramUtxo,
  PlaceHolderTData,
} from "../utxo";
import { Provider } from "../provider";
import { Account } from "../account";

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
  recentTransactionParameters?: Transaction;
  recentTransaction?: Transaction;
  approved?: boolean;
  appUtxoConfig?: AppUtxoConfig;
  balance: Balance;
  inboxBalance: InboxBalance;
  verifierIdl: Idl;
  recentInstructions?: TransactionInstruction[];

  constructor({
    provider,
    account,
    appUtxoConfig,
    verifierIdl = IDL_LIGHT_PSP2IN2OUT,
  }: {
    provider: Provider;
    serializedUtxos?: Buffer;
    serializedSpentUtxos?: Buffer;
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

    if (
      !provider.lookUpTables.verifierProgramLookupTable ||
      !provider.lightWasm
    )
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
    this.verifierIdl = verifierIdl ? verifierIdl : IDL_LIGHT_PSP2IN2OUT;
    this.balance = {
      tokenBalances: new Map([
        [SystemProgram.programId.toBase58(), TokenUtxoBalance.initSol()],
      ]),
      programBalances: new Map(),
      nftBalances: new Map(),
      totalSolBalance: BN_0,
    };
    this.inboxBalance = {
      tokenBalances: new Map([
        [SystemProgram.programId.toBase58(), TokenUtxoBalance.initSol()],
      ]),
      programBalances: new Map(),
      nftBalances: new Map(),
      numberInboxUtxos: 0,
      totalSolBalance: BN_0,
    };
  }

  // TODO: should update merkle tree as well
  // TODO: test robustness
  // TODO: nonce incrementing is very ugly revisit
  async syncState(
    aes: boolean = true,
    balance: Balance | InboxBalance,
  ): Promise<Balance | InboxBalance> {
    // reduce balance by spent utxos
    if (!this.provider.provider)
      throw new UserError(
        UserErrorCode.PROVIDER_NOT_INITIALIZED,
        "syncState",
        "provider is undefined",
      );

    // identify spent utxos
    for (const [, tokenBalance] of balance.tokenBalances) {
      for (const [key, utxo] of tokenBalance.utxos) {
        const nullifierAccountInfo = await fetchNullifierAccountInfo(
          utxo.nullifier,
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

    const prefixes = aes
      ? getPrefixes(this.account, this.provider.rpc.accounts.merkleTreeSet, 100)
      : [bs58.encode(this.account.encryptionKeypair.publicKey.slice(0, 4))];
    const indexedTransactions = await this.provider.rpc.getEventsByIdBatch(
      prefixes,
      0,
    );
    if (!indexedTransactions) return balance;

    for (const trx of indexedTransactions) {
      const leftLeafIndex = new BN(
        trx.transaction.firstLeafIndex,
        "hex",
      ).toNumber();

      for (let index = 0; index < trx.transaction.leaves.length; index += 2) {
        const leafLeft = trx.transaction.leaves[index];
        const leafRight = trx.transaction.leaves[index + 1];
        const encUtxoSize =
          NACL_ENCRYPTED_COMPRESSED_UTXO_BYTES_LENGTH + UTXO_PREFIX_LENGTH;
        // transaction nonce is the same for all utxos in one transaction
        await decryptAddUtxoToBalance({
          encBytes: Buffer.from(
            trx.transaction.encryptedUtxos.slice(
              index * encUtxoSize,
              index * encUtxoSize + encUtxoSize,
            ),
          ),
          index: leftLeafIndex,
          commitment: Buffer.from([...leafLeft]),
          account: this.account,
          lightWasm: this.provider.lightWasm,
          connection: this.provider.provider.connection,
          balance,
          merkleTreePdaPublicKey: this.provider.rpc.accounts.merkleTreeSet,
          leftLeaf: Uint8Array.from([...leafLeft]),
          aes,
          assetLookupTable: this.provider.lookUpTables.assetLookupTable,
          merkleProof: trx.merkleProofs[index],
        });
        await decryptAddUtxoToBalance({
          encBytes: Buffer.from(
            trx.transaction.encryptedUtxos.slice(
              index * encUtxoSize + encUtxoSize,
              index * encUtxoSize + encUtxoSize * 2,
            ),
          ),
          index: leftLeafIndex + 1,
          commitment: Buffer.from([...leafRight]),
          account: this.account,
          lightWasm: this.provider.lightWasm,
          connection: this.provider.provider.connection,
          balance,
          merkleTreePdaPublicKey: this.provider.rpc.accounts.merkleTreeSet,
          leftLeaf: Uint8Array.from([...leafLeft]),
          aes,
          assetLookupTable: this.provider.lookUpTables.assetLookupTable,
          merkleProof: trx.merkleProofs[index + 1],
        });
      }
    }

    // calculate total sol balance
    const calculateTotalSolBalance = (balance: Balance) => {
      let totalSolBalance = BN_0;
      for (const tokenBalance of balance.tokenBalances.values()) {
        totalSolBalance = totalSolBalance.add(tokenBalance.totalBalanceSol);
      }
      return totalSolBalance;
    };

    this.transactionHistory = await getUserIndexTransactions(
      indexedTransactions.map((trx) => trx.transaction),
      this.provider,
      this.balance.tokenBalances,
    );

    balance.totalSolBalance = calculateTotalSolBalance(balance);
    return balance;
  }

  /**
   * returns all non-accepted utxos.
   * would not be part of the main balance
   */
  async getUtxoInbox(latest: boolean = true): Promise<InboxBalance> {
    if (latest) {
      await this.syncState(false, this.inboxBalance);
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
    if (!this.provider.lookUpTables.verifierProgramLookupTable)
      throw new UserError(
        RpcErrorCode.LOOK_UP_TABLE_UNDEFINED,
        "getBalance",
        "Look up table not initialized",
      );

    if (latest) {
      await this.syncState(true, this.balance);
    }
    return this.balance;
  }

  /**
   *
   * @param amount e.g. 1 SOL = 1, 2 USDC = 2
   * @param token "SOL", "USDC", "USDT",
   * @param recipient optional, if not set, will compress to self
   * @param senderTokenAccount optional, if set, will use this token account to compress from, else derives ATA
   */
  async createCompressTransactionParameters({
    token,
    publicAmountSpl,
    recipient,
    publicAmountSol,
    senderTokenAccount,
    minimumLamports = true,
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
    mergeExistingUtxos?: boolean;
    verifierIdl?: Idl;
    message?: Buffer;
    skipDecimalConversions?: boolean;
    utxo?: OutUtxo;
  }): Promise<Transaction> {
    // TODO: add errors for if appUtxo appDataHash or appData, no verifierAddress
    if (publicAmountSpl && token === "SOL")
      throw new UserError(
        UserErrorCode.INVALID_TOKEN,
        "compress",
        "No public amount provided. Compress needs a public amount.",
      );
    if (!publicAmountSpl && !publicAmountSol)
      throw new UserError(
        CreateUtxoErrorCode.NO_PUBLIC_AMOUNTS_PROVIDED,
        "compress",
        "No public amounts provided. Compress needs a public amount.",
      );

    if (publicAmountSpl && !token)
      throw new UserError(
        UserErrorCode.TOKEN_UNDEFINED,
        "compress",
        "No public amounts provided. Compress needs a public amount.",
      );

    if (!this.provider)
      throw new UserError(
        UserErrorCode.PROVIDER_NOT_INITIALIZED,
        "compress",
        "Provider not set!",
      );
    const tokenCtx = TOKEN_REGISTRY.get(token);

    if (!tokenCtx)
      throw new UserError(
        UserErrorCode.TOKEN_NOT_FOUND,
        "compress",
        "Token not supported!",
      );
    if (tokenCtx.isNative && senderTokenAccount)
      throw new UserError(
        UserErrorCode.TOKEN_ACCOUNT_DEFINED,
        "compress",
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
      : BN_0;
    publicAmountSpl = convertedPublicAmounts.publicAmountSpl;

    if (publicAmountSol && this.provider.connection) {
      const solBalance = await this.provider.connection.getBalance(
        this.provider.wallet.publicKey,
      );
      if (solBalance < publicAmountSol.toNumber()) {
        throw new UserError(
          UserErrorCode.INSUFFICIENT_BAlANCE,
          "createCompressTransactionParameters",
          `The current balance is insufficient for this operation. The user's balance is ${
            solBalance / LAMPORTS_PER_SOL
          } SOL, but the operation requires ${
            publicAmountSol.toNumber() / LAMPORTS_PER_SOL
          } SOL.`,
        );
      }
    }

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
    const utxosEntries = this.balance.tokenBalances
      .get(tokenCtx.mint.toBase58())
      ?.utxos.values();
    let utxos =
      utxosEntries && mergeExistingUtxos ? Array.from(utxosEntries) : [];
    const outUtxos: OutUtxo[] = [];
    if (recipient) {
      const amounts: BN[] = publicAmountSpl
        ? [publicAmountSol, publicAmountSpl]
        : [publicAmountSol];
      const assets = !tokenCtx.isNative
        ? [SystemProgram.programId, tokenCtx.mint]
        : [SystemProgram.programId];
      outUtxos.push(
        createOutUtxo({
          lightWasm: this.provider.lightWasm,
          assets,
          amounts,
          owner: recipient.keypair.publicKey,
          encryptionPublicKey: recipient.encryptionKeypair.publicKey,
          isPublic: false,
        }),
      );
      utxos = [];
    }
    if (utxo) outUtxos.push(utxo);
    const txParams = await getTxParams({
      tokenCtx,
      action: Action.COMPRESS,
      account: this.account,
      utxos,
      publicAmountSol,
      publicAmountSpl,
      userSplAccount,
      provider: this.provider,
      verifierIdl: verifierIdl ? verifierIdl : this.verifierIdl,
      outUtxos,
      addInUtxos: !recipient,
      addOutUtxos: !recipient,
      message,
      assetLookupTable: this.provider.lookUpTables.assetLookupTable,
      verifierProgramLookupTable:
        this.provider.lookUpTables.verifierProgramLookupTable,
      rpc: this.provider.rpc,
    });
    this.recentTransactionParameters = txParams;
    return txParams;
  }

  async compileAndProveTransaction(
    shuffleEnabled: boolean = true,
  ): Promise<TransactionInstruction[]> {
    if (!this.recentTransactionParameters)
      throw new UserError(
        UserErrorCode.TRANSACTION_PARAMETERS_UNDEFINED,
        "compileAndProveTransaction",
        "The method 'createCompressTransactionParameters' must be executed first to generate the parameters that can be compiled and proven.",
      );
    let root: string | undefined = undefined;
    let rootIndex: number | undefined = undefined;
    if (
      this.recentTransactionParameters.private.inputUtxos &&
      this.recentTransactionParameters.private.inputUtxos.length != 0
    ) {
      const {
        syncedUtxos,
        root: fetchedRoot,
        index,
      } = await syncInputUtxosMerkleProofs({
        inputUtxos: this.recentTransactionParameters.private.inputUtxos,
        rpc: this.provider.rpc,
      });
      this.recentTransactionParameters.private.inputUtxos = syncedUtxos;
      root = fetchedRoot;
      rootIndex = index;
    } else {
      const res = (await this.provider.rpc!.getMerkleRoot(MERKLE_TREE_SET))!;
      root = res.root;
      rootIndex = res.index;
    }
    if (!root) {
      throw new TransactionParametersError(
        TransactionParametersErrorCode.FETCHING_ROOT_FAILED,
        "getTxParams",
        "Fetching root from rpc failed.",
      );
    }
    const systemProofInputs = createSystemProofInputs({
      transaction: this.recentTransactionParameters,
      lightWasm: this.provider.lightWasm,
      account: this.account,
      root,
    });
    const systemProof = await getSystemProof({
      account: this.account,
      inputUtxos: this.recentTransactionParameters.private.inputUtxos,
      verifierIdl: getSystemPspIdl(
        this.recentTransactionParameters.public.accounts.systemPspId,
      )!,
      systemProofInputs,
    });
    const remainingSolanaAccounts = getSolanaRemainingAccounts(
      systemProof.parsedPublicInputsObject,
      // TODO: readd remainingMerkleTreeAccounts,
    );
    const accounts = prepareAccounts({
      transactionAccounts: this.recentTransactionParameters.public.accounts,
      merkleTreeSet: this.provider.rpc.accounts.merkleTreeSet,
      rpcRecipientSol: this.provider.rpc.accounts.rpcRecipientSol,
      signer: this.recentTransactionParameters.public.accounts.rpcPublicKey,
    });

    this.recentInstructions = await createSolanaInstructions({
      action: this.recentTransactionParameters["action"]
        ? this.recentTransactionParameters["action"]
        : Action.TRANSFER,
      systemProof,
      remainingSolanaAccounts,
      accounts,
      publicTransactionVariables: this.recentTransactionParameters.public,
      systemPspIdl: getSystemPspIdl(
        this.recentTransactionParameters.public.accounts.systemPspId,
      ),
      rootIndex,
    });
    return this.recentInstructions;
  }

  async approve() {
    if (!this.recentTransactionParameters)
      throw new UserError(
        UserErrorCode.TRANSACTION_PARAMETERS_UNDEFINED,
        "compileAndProveTransaction",
        "The method 'createCompressTransactionParameters' must be executed first to approve SPL funds before initiating a compress transaction.",
      );
    if (
      this.recentTransactionParameters?.public.publicAmountSpl.gt(BN_0) &&
      this.recentTransactionParameters["action"] === Action.COMPRESS
    ) {
      const tokenAccountInfo =
        await this.provider.provider?.connection!.getAccountInfo(
          this.recentTransactionParameters.public.accounts.senderSpl!,
        );

      if (!tokenAccountInfo)
        throw new UserError(
          UserErrorCode.ASSOCIATED_TOKEN_ACCOUNT_DOESNT_EXIST,
          "compress",
          "AssociatedTokenAccount doesn't exist!",
        );
      const tokenBalance = await splToken.getAccount(
        this.provider.provider.connection!,
        this.recentTransactionParameters.public.accounts.senderSpl!,
      );

      if (
        this.recentTransactionParameters?.public.publicAmountSpl.gt(
          new BN(tokenBalance.amount.toString()),
        )
      )
        throw new UserError(
          UserErrorCode.INSUFFICIENT_BAlANCE,
          "compress",
          `Insufficient token balance! ${this.recentTransactionParameters?.public.publicAmountSpl.toString()} bal: ${tokenBalance!
            .amount!}`,
        );

      try {
        const transaction = new SolanaTransaction().add(
          splToken.createApproveInstruction(
            this.recentTransactionParameters.public.accounts.senderSpl!,
            AUTHORITY,
            this.provider.wallet!.publicKey,
            this.recentTransactionParameters?.public.publicAmountSpl.toNumber(),
          ),
        );
        transaction.recentBlockhash = (
          await this.provider.provider?.connection.getLatestBlockhash(
            "confirmed",
          )
        )?.blockhash;
        await this.provider.wallet!.sendAndConfirmTransaction(transaction);
        this.approved = true;
      } catch (e: any) {
        throw new UserError(
          UserErrorCode.APPROVE_ERROR,
          "compress",
          `Error approving token transfer! ${e.stack}`,
        );
      }
    } else {
      this.approved = true;
    }
  }

  async sendTransaction() {
    if (!this.recentTransactionParameters)
      throw new UserError(
        UserErrorCode.TRANSACTION_PARAMETERS_UNDEFINED,
        "sendTransaction",
        "Unable to send transaction. The transaction must be compiled and a proof must be generated first.",
      );
    if (
      this.recentTransactionParameters["action"] === Action.COMPRESS &&
      !this.approved
    )
      throw new UserError(
        UserErrorCode.SPL_FUNDS_NOT_APPROVED,
        "sendTransaction",
        "Please approve SPL funds before executing a compression with SPL tokens.",
      );
    if (!this.recentInstructions)
      throw new UserError(
        UserErrorCode.INSTRUCTIONS_UNDEFINED,
        "sendTransaction",
        "Unable to send transaction. The transaction must be compiled and a proof must be generated first to create solana instructions.",
      );
    let txResult;
    try {
      if (this.recentTransactionParameters["action"] === Action.COMPRESS) {
        txResult = await this.provider.sendAndConfirmSolanaInstructions(
          this.recentInstructions,
        );
      } else {
        txResult = await this.provider.rpc.sendAndConfirmSolanaInstructions(
          this.recentInstructions,
          this.provider.provider.connection,
          undefined,
          undefined,
          this.provider,
        );
      }
    } catch (e) {
      throw new UserError(
        TransactionErrorCode.SEND_TRANSACTION_FAILED,
        "compress",
        `Error in tx.sendTransaction ${e}`,
      );
    }
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
   * @param recipient optional, if not set, will compress to self
   * @param senderTokenAccount optional, if set, will use this token account to compress from, else derives ATA
   */
  async compress({
    token,
    publicAmountSpl,
    recipient,
    publicAmountSol,
    senderTokenAccount,
    minimumLamports = true,
    skipDecimalConversions = false,
    confirmOptions = ConfirmOptions.spendable,
  }: {
    token: string;
    recipient?: string;
    publicAmountSpl?: number | BN | string;
    publicAmountSol?: number | BN | string;
    minimumLamports?: boolean;
    senderTokenAccount?: PublicKey;
    skipDecimalConversions?: boolean;
    confirmOptions?: ConfirmOptions;
  }): Promise<ActionResponseMulti> {
    const recipientAccount = recipient
      ? Account.fromPubkey(recipient, this.provider.lightWasm)
      : undefined;

    const txParams = await this.createCompressTransactionParameters({
      token,
      publicAmountSpl,
      recipient: recipientAccount,
      publicAmountSol,
      senderTokenAccount,
      minimumLamports,
      skipDecimalConversions,
    });
    return await this.transactWithParameters({ txParams, confirmOptions });
  }

  async decompress({
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
  }): Promise<ActionResponseMulti> {
    const txParams = await this.createDecompressTransactionParameters({
      token,
      publicAmountSpl,
      publicAmountSol,
      recipient,
      minimumLamports,
    });
    return await this.transactWithParameters({ txParams, confirmOptions });
  }

  // TODO: add decompressSol and decompressSpl
  // TODO: add optional pass-in token mint
  // TODO: add pass-in mint
  /**
   * @params token: string
   * @params amount: number - in base units (e.g. lamports for 'SOL')
   * @params recipient: PublicKey - Solana address
   * @params extraSolAmount: number - optional, if not set, will use MINIMUM_LAMPORTS
   */
  async createDecompressTransactionParameters({
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
        "decompress",
        "Token not supported!",
      );

    if (!publicAmountSpl && !publicAmountSol)
      throw new UserError(
        CreateUtxoErrorCode.NO_PUBLIC_AMOUNTS_PROVIDED,
        "decompress",
        "Need to provide at least one amount for a decompression",
      );
    if (publicAmountSol && recipient.toBase58() == AUTHORITY.toBase58())
      throw new UserError(
        TransactionErrorCode.SOL_RECIPIENT_UNDEFINED,
        "getTxParams",
        "Please provide a recipient for decompressing SOL",
      );
    if (publicAmountSpl && recipient.toBase58() == AUTHORITY.toBase58())
      throw new UserError(
        TransactionErrorCode.SPL_RECIPIENT_UNDEFINED,
        "getTxParams",
        "Please provide a recipient for decompressing SPL",
      );
    if (publicAmountSpl && token == "SOL")
      throw new UserError(
        UserErrorCode.INVALID_TOKEN,
        "getTxParams",
        "public amount spl provided for SOL token",
      );

    let ataCreationFee = false;
    let recipientSpl: PublicKey | undefined;

    if (publicAmountSpl) {
      recipientSpl = splToken.getAssociatedTokenAddressSync(
        tokenCtx!.mint,
        recipient,
      );
      const tokenAccountInfo =
        await this.provider.provider!.connection?.getAccountInfo(recipientSpl);
      if (!tokenAccountInfo) {
        ataCreationFee = true;
      }
    }

    let _publicSplAmount: BN | undefined = undefined;
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
      : BN_0;
    const utxosEntries = this.balance.tokenBalances
      .get(tokenCtx.mint.toBase58())
      ?.utxos.values();
    const solUtxos = this.balance.tokenBalances
      .get(SystemProgram.programId.toBase58())
      ?.utxos.values();

    const utxosEntriesSol =
      solUtxos && !tokenCtx.isNative ? Array.from(solUtxos) : new Array<Utxo>();

    const utxos = utxosEntries
      ? Array.from([...utxosEntries, ...utxosEntriesSol])
      : [];

    const txParams = await getTxParams({
      tokenCtx,
      publicAmountSpl: _publicSplAmount,
      action: Action.DECOMPRESS,
      account: this.account,
      utxos,
      publicAmountSol: _publicSolAmount,
      recipientSol: recipient,
      recipientSplAddress: recipientSpl,
      provider: this.provider,
      rpc: this.provider.rpc,
      ataCreationFee,
      appUtxo: this.appUtxoConfig,
      verifierIdl: IDL_LIGHT_PSP2IN2OUT,
      assetLookupTable: this.provider.lookUpTables.assetLookupTable,
      verifierProgramLookupTable:
        this.provider.lookUpTables.verifierProgramLookupTable,
    });
    this.recentTransactionParameters = txParams;
    return txParams;
  }

  // TODO: replace recipient with recipient light public key
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
  }): Promise<ActionResponseMulti> {
    if (!recipient)
      throw new UserError(
        UserErrorCode.COMPRESSED_RECIPIENT_UNDEFINED,
        "transfer",
        "Please provide a compressed recipient for the transfer.",
      );
    const recipientAccount = Account.fromPubkey(
      recipient,
      this.provider.lightWasm,
    );

    const txParams = await this.createTransferTransactionParameters({
      token,
      recipient: recipientAccount,
      amountSpl,
      amountSol,
      appUtxo,
    });
    return this.transactWithParameters({ txParams, confirmOptions });
  }

  // TODO: add separate lookup function for users.
  // TODO: add account parsing from and to string which is concat compressed pubkey and encryption key
  /**
   * @description transfers to one recipient utxo and crencrypteates a change utxo with remainders of the input
   * @param token mint
   * @param amount
   * @param recipient compressedAddress (BN)
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
    outUtxos?: OutUtxo[];
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
    const parsedSolAmount = convertedPublicAmounts.publicAmountSol
      ? convertedPublicAmounts.publicAmountSol
      : BN_0;
    const parsedSplAmount = convertedPublicAmounts.publicAmountSpl
      ? convertedPublicAmounts.publicAmountSpl
      : BN_0;

    if (recipient && !tokenCtx)
      throw new UserError(
        UserErrorCode.COMPRESSED_RECIPIENT_UNDEFINED,
        "createTransferTransactionParameters",
      );

    let _outUtxos: OutUtxo[] = [];
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
        assetLookupTable: this.provider.lookUpTables.assetLookupTable,
        lightWasm: this.provider.lightWasm,
        isPublic: false,
      });
    }

    if (outUtxos) _outUtxos = [..._outUtxos, ...outUtxos];

    const solUtxos = this.balance.tokenBalances
      .get(SystemProgram.programId.toBase58())
      ?.utxos.values();
    const utxosEntriesSol =
      solUtxos && token !== "SOL" ? Array.from(solUtxos) : new Array<Utxo>();

    const utxosEntries = this.balance.tokenBalances
      .get(tokenCtx.mint.toBase58())
      ?.utxos.values();
    const utxos = utxosEntries
      ? Array.from([...utxosEntries, ...utxosEntriesSol])
      : [];

    if (!tokenCtx.isNative && !utxosEntries)
      throw new UserError(
        UserErrorCode.INSUFFICIENT_BAlANCE,
        "createTransferTransactionParameters",
        `Balance does not have any utxos of ${token}`,
      );

    const txParams = await getTxParams({
      tokenCtx,
      action: Action.TRANSFER,
      account: this.account,
      utxos,
      inUtxos,
      outUtxos: _outUtxos,
      provider: this.provider,
      rpc: this.provider.rpc,
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
    confirmOptions,
    shuffleEnabled = true,
  }: {
    txParams: Transaction;
    confirmOptions?: ConfirmOptions;
    shuffleEnabled?: boolean;
  }): Promise<ActionResponseMulti> {
    this.recentTransactionParameters = txParams;

    this.recentInstructions =
      await this.compileAndProveTransaction(shuffleEnabled);

    await this.approve();
    this.approved = true;

    // we send an array of instructions to the rpc and the rpc sends 3 transaction
    const txHash = await this.sendTransaction();

    const rpcMerkleTreeUpdateResponse = "success";

    await this.getBalance();

    this.resetTxState();
    return {
      txHash: { signatures: txHash }, // TODO: unify external interfaces
      response: rpcMerkleTreeUpdateResponse,
    };
  }

  // @ts-ignore
  async transactWithUtxos({
    action,
    inUtxoCommitments,
    inUtxos,
    outUtxos,
  }: {
    action: Action;
    inUtxoCommitments: string[];
    inUtxos: Utxo[];
    outUtxos: Utxo[];
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
    appUtxoConfig,
    account,
    skipFetchBalance,
  }: {
    provider: Provider;
    seed?: string;
    utxos?: Utxo[];
    appUtxoConfig?: AppUtxoConfig;
    account?: Account;
    skipFetchBalance?: boolean;
  }): Promise<User> {
    try {
      if (seed && account)
        throw new UserError(
          UserErrorCode.ACCOUNT_AND_SEED_PROVIDED,
          "load",
          "Cannot provide both seed and account!",
        );
      if (!seed && !account && provider.wallet) {
        account = await Account.createFromBrowserWallet(
          provider.lightWasm,
          provider.wallet,
        );
      } else if (!account && seed) {
        account = Account.createFromSeed(provider.lightWasm, seed);
      } else if (!account) {
        throw new UserError(
          CreateUtxoErrorCode.ACCOUNT_UNDEFINED,
          "load",
          "No account, provider with wallet or seed provided!",
        );
      }
      if (
        account.solanaPublicKey &&
        provider.wallet.publicKey.toBase58() !==
          account.solanaPublicKey.toBase58()
      ) {
        console.warn(
          "Account seems to be created from a different wallet than provider uses.",
        );
      }
      const user = new User({ provider, appUtxoConfig, account });
      if (!skipFetchBalance) await user.getBalance();

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
  /** compressed transfer to self, merge 10-1;
   * get utxo inbox
   * merge highest first
   * loops in steps of 9 or 10
   */
  async mergeAllUtxos(
    asset: PublicKey,
    confirmOptions: ConfirmOptions = ConfirmOptions.spendable,
    latest: boolean = true,
  ): Promise<ActionResponseMulti> {
    await this.getUtxoInbox(latest);
    await this.getBalance(latest);
    const inboxTokenBalance: TokenUtxoBalance | undefined =
      this.inboxBalance.tokenBalances.get(asset.toString());
    if (!inboxTokenBalance)
      throw new UserError(
        UserErrorCode.EMPTY_INBOX,
        "mergeAllUtxos",
        `for asset ${asset} the utxo inbox is empty`,
      );

    const utxosEntries = this.balance.tokenBalances
      .get(asset.toBase58())
      ?.utxos.values();
    const _solUtxos = this.balance.tokenBalances
      .get(new PublicKey(0).toBase58())
      ?.utxos.values();
    const solUtxos = _solUtxos ? Array.from(_solUtxos) : [];
    const inboxUtxosEntries = Array.from(inboxTokenBalance.utxos.values());

    if (inboxUtxosEntries.length == 0)
      throw new UserError(
        UserErrorCode.EMPTY_INBOX,
        "mergeAllUtxos",
        `for asset ${asset} the utxo inbox is empty`,
      );
    const assetIndex =
      asset.toBase58() === SystemProgram.programId.toBase58() ? 0 : 1;
    // sort inbox utxos descending
    inboxUtxosEntries.sort(
      (a, b) =>
        b.amounts[assetIndex].toNumber() - a.amounts[assetIndex].toNumber(),
    );

    let inUtxos =
      utxosEntries && asset.toBase58() !== new PublicKey(0).toBase58()
        ? Array.from([...utxosEntries, ...solUtxos, ...inboxUtxosEntries])
        : Array.from([...solUtxos, ...inboxUtxosEntries]);
    if (inUtxos.length > 10) {
      inUtxos = inUtxos.slice(0, 10);
    }

    const txParams = await getTxParams({
      tokenCtx: inboxTokenBalance.tokenData,
      action: Action.TRANSFER,
      provider: this.provider,
      inUtxos,
      addInUtxos: false,
      addOutUtxos: true,
      separateSolUtxo: true,
      account: this.account,
      mergeUtxos: true,
      rpc: this.provider.rpc,
      verifierIdl: IDL_LIGHT_PSP10IN2OUT,
      assetLookupTable: this.provider.lookUpTables.assetLookupTable,
      verifierProgramLookupTable:
        this.provider.lookUpTables.verifierProgramLookupTable,
    });
    return await this.transactWithParameters({ txParams, confirmOptions });
  }

  // TODO: how do we handle app utxos?, some will not be able to be accepted we can only mark these as accepted
  /** compressed transfer to self, merge 10-1;
   * get utxo inbox
   * merge highest first
   * loops in steps of 9 or 10
   */
  async mergeUtxos(
    commitments: string[],
    asset: PublicKey,
    confirmOptions: ConfirmOptions = ConfirmOptions.spendable,
    latest: boolean = false,
  ): Promise<ActionResponseMulti> {
    if (commitments.length == 0)
      throw new UserError(
        UserErrorCode.NO_COMMITMENTS_PROVIDED,
        "mergeAllUtxos",
        `No commitments for merging specified ${asset}`,
      );

    await this.getUtxoInbox(latest);
    await this.getBalance(latest);
    const inboxTokenBalance: TokenUtxoBalance | undefined =
      this.inboxBalance.tokenBalances.get(asset.toString());
    if (!inboxTokenBalance)
      throw new UserError(
        UserErrorCode.EMPTY_INBOX,
        "mergeAllUtxos",
        `for asset ${asset} the utxo inbox is empty`,
      );

    const utxosEntries = this.balance.tokenBalances
      .get(asset.toBase58())
      ?.utxos.values();

    const commitmentUtxos: (Utxo | ProgramUtxo<PlaceHolderTData>)[] = [];
    for (const commitment of commitments) {
      const utxo = inboxTokenBalance.utxos.get(commitment);
      if (!utxo)
        throw new UserError(
          UserErrorCode.COMMITMENT_NOT_FOUND,
          "mergeUtxos",
          `commitment ${commitment} is it of asset ${asset} ?`,
        );
      commitmentUtxos.push(utxo);
    }

    const inUtxos = utxosEntries
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

    const txParams = await getTxParams({
      tokenCtx: inboxTokenBalance.tokenData,
      action: Action.TRANSFER,
      provider: this.provider,
      inUtxos,
      addInUtxos: false,
      addOutUtxos: true,
      account: this.account,
      mergeUtxos: true,
      rpc: this.provider.rpc,
      verifierIdl: IDL_LIGHT_PSP10IN2OUT,
      assetLookupTable: this.provider.lookUpTables.assetLookupTable,
      verifierProgramLookupTable:
        this.provider.lookUpTables.verifierProgramLookupTable,
    });
    return this.transactWithParameters({ txParams, confirmOptions });
  }

  async getTransactionHistory(
    latest: boolean = true,
  ): Promise<UserIndexedTransaction[]> {
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

  // getPrivacyScore() -> for decompress only, can separate into its own helper method
  // Fetch utxos should probably be a function such the user object is not occupied while fetching,
  // but it would probably be more logical to fetch utxos here as well
  addUtxos() {
    throw new Error("not implemented yet");
  }

  // TODO: add storage transaction nonce to rotate keypairs
  /**
   * - get indexed transactions for a storage compressed account
   * - try to decrypt all and add to appUtxos or decrypted data map
   * - add custom description strategies for arbitrary data
   */
  async syncStorage(idl: Idl, aes: boolean = true) {
    // TODO: move to rpc
    // TODO: implement the following
    /**
     * get all transactions of the storage verifier and filter for the ones including noop program
     * build merkle tree and check versus root on-chain
     * mark as cleartext and as decrypted with the first byte
     * [
     *  1 bytes: encrypted or cleartext 1 byte,
     *  32bytes:  encryptionAlgo/Mode,
     *  remaining message
     * ]
     */

    const prefixes = aes
      ? getPrefixes(this.account, this.provider.rpc.accounts.merkleTreeSet, 100)
      : [bs58.encode(this.account.encryptionKeypair.publicKey.slice(0, 4))];
    const indexedTransactions = await this.provider.rpc.getEventsByIdBatch(
      prefixes,
      0,
    );
    if (!indexedTransactions)
      throw new UserError(
        UserErrorCode.FETCHING_INDEXED_TRANSACTIONS_FAILED,
        "syncStorage",
        "Fetching indexed transactions from rpc failed!",
      );

    const indexedStorageVerifierTransactionsFiltered =
      indexedTransactions.filter((indexedTransaction) => {
        return indexedTransaction.transaction.message.length !== 0;
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
    // TODO: support decryption of multiple messages in one transaction, the first 8 bytes should be the length of the message
    const decryptIndexStorage = async (
      indexedTransactions: RpcIndexedTransactionResponse[],
      assetLookupTable: string[],
      aes: boolean,
    ) => {
      const decryptedStorageUtxos: ProgramUtxo<PlaceHolderTData>[] = [];
      const spentUtxos: ProgramUtxo<PlaceHolderTData>[] = [];
      for (const data of indexedTransactions) {
        let decryptedUtxo: Result<
          ProgramUtxo<PlaceHolderTData> | null,
          UtxoError
        >;
        let index = new BN(data.transaction.firstLeafIndex, "hex").toNumber();
        for (const [, leaf] of data.transaction.leaves.entries()) {
          try {
            decryptedUtxo = await decryptProgramUtxo({
              lightWasm: this.provider.lightWasm,
              account: this.account,
              encBytes: Uint8Array.from(data.transaction.message),
              ownerIdl: idl,
              owner: getVerifierProgramId(idl),
              type: "utxo", // TODO: try all accounts which are appended with OutUtxo from idl
              aes,
              merkleTreeLeafIndex: index,
              utxoHash: Uint8Array.from(leaf),
              merkleTreePdaPublicKey: this.provider.rpc.accounts.merkleTreeSet,
              compressed: false,
              assetLookupTable,
              merkleProof: data.merkleProofs[index],
            });

            if (decryptedUtxo.value) {
              const utxo = decryptedUtxo.value;
              const nfExists = await fetchNullifierAccountInfo(
                utxo.nullifier,
                this.provider.provider?.connection,
              );

              if (!nfExists) {
                decryptedStorageUtxos.push(utxo);
              } else {
                spentUtxos.push(utxo);
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
      aes,
    );

    for (const utxo of decryptedStorageUtxos) {
      const verifierAddress = getVerifierProgramId(idl).toBase58();
      if (!this.balance.programBalances.get(verifierAddress)) {
        this.balance.programBalances.set(
          verifierAddress,
          new ProgramUtxoBalance(getVerifierProgramId(idl), idl),
        );
      }
      this.balance.programBalances
        .get(verifierAddress)!
        .addUtxo(utxo.hash.toString(), utxo, "utxos");
    }

    for (const utxo of spentUtxos) {
      const verifierAddress = getVerifierProgramId(idl).toBase58();
      if (!this.balance.programBalances.get(verifierAddress)) {
        this.balance.programBalances.set(
          verifierAddress,
          new ProgramUtxoBalance(getVerifierProgramId(idl), idl),
        );
      }
      this.balance.programBalances
        .get(verifierAddress)!
        .addUtxo(utxo.hash.toString(), utxo, "spentUtxos");
    }
    for (const [, programBalance] of this.balance.programBalances) {
      for (const [, tokenBalance] of programBalance.tokenBalances) {
        for (const [key, utxo] of tokenBalance.utxos) {
          const nullifierAccountInfo = await fetchNullifierAccountInfo(
            utxo.nullifier,
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

  getAllUtxos(): (Utxo | ProgramUtxo<PlaceHolderTData>)[] {
    const allUtxos: (Utxo | ProgramUtxo<PlaceHolderTData>)[] = [];

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
    compress: boolean = false,
  ) {
    if (message.length > MAX_MESSAGE_SIZE)
      throw new UserError(
        UserErrorCode.MAX_STORAGE_MESSAGE_SIZE_EXCEEDED,
        "storeData",
        `${message.length}/${MAX_MESSAGE_SIZE}`,
      );
    if (compress) {
      this.recentTransactionParameters =
        await this.createCompressTransactionParameters({
          token: "SOL",
          publicAmountSol: BN_0,
          minimumLamports: false,
          message,
          verifierIdl: IDL_LIGHT_PSP2IN2OUT_STORAGE,
        });
    } else {
      const inUtxos: (Utxo | ProgramUtxo<PlaceHolderTData>)[] = [];
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
        const allUtxos = this.getAllUtxos();
        allUtxos.sort((a, b) => a.amounts[0].sub(b.amounts[0]).toNumber());
        inUtxos.push(allUtxos[0]);
      }
      if (inUtxos.length === 0 || inUtxos[0] === undefined)
        throw new UserError(
          SelectInUtxosErrorCode.FAILED_TO_SELECT_SOL_UTXO,
          "storeData",
        );

      const tokenCtx = TOKEN_REGISTRY.get("SOL")!;

      this.recentTransactionParameters = await getTxParams({
        tokenCtx,
        action: Action.TRANSFER,
        account: this.account,
        inUtxos,
        provider: this.provider,
        rpc: this.provider.rpc,
        appUtxo: this.appUtxoConfig,
        message,
        mergeUtxos: true,
        addInUtxos: false,
        verifierIdl: IDL_LIGHT_PSP2IN2OUT_STORAGE,
        assetLookupTable: this.provider.lookUpTables.assetLookupTable,
        verifierProgramLookupTable:
          this.provider.lookUpTables.verifierProgramLookupTable,
      });
    }

    return this.transactWithParameters({
      txParams: this.recentTransactionParameters!,
      confirmOptions,
    });
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
    merkleTree?: PublicKey;
  }) {
    const programAddress = getVerifierProgramId(idl);
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
    const programUtxoArray: (Utxo | ProgramUtxo<PlaceHolderTData>)[] = [];
    if (programBalance) {
      for (const tokenBalance of programBalance.tokenBalances.values()) {
        programUtxoArray.push(...tokenBalance.utxos.values());
      }
    }
    const inboxProgramUtxoArray: (Utxo | ProgramUtxo<PlaceHolderTData>)[] = [];
    if (inboxProgramBalance) {
      for (const tokenBalance of inboxProgramBalance.tokenBalances.values()) {
        inboxProgramUtxoArray.push(...tokenBalance.utxos.values());
      }
    }
    return { programUtxoArray, inboxProgramUtxoArray };
  }

  async getUtxo(
    commitment: string,
    latest: boolean = false,
    idl?: Idl,
  ): Promise<
    { utxo: Utxo | ProgramUtxo<PlaceHolderTData>; status: string } | undefined
  > {
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
      for (const [, tokenBalance] of tokenBalances) {
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
    for (const [, programBalance] of this.balance.programBalances) {
      const res = iterateOverTokenBalance(programBalance.tokenBalances);
      if (res) return res;
    }
    return iterateOverTokenBalance(this.balance.tokenBalances);
  }
}

// get prefixes until the latest leaf index
export const getPrefixes = (
  account: Account,
  merkleTreePdaPublicKey: PublicKey,
  no: number,
) => {
  const prefixes: string[] = [];
  for (let i = 0; i < no; i++) {
    const prefix = account.generateUtxoPrefixHash(merkleTreePdaPublicKey, i);
    prefixes.push(bs58.encode(prefix));
  }
  return prefixes;
};
