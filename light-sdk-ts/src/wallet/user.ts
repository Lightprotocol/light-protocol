import {
  Keypair as SolanaKeypair,
  PublicKey,
  SystemProgram,
  Transaction as createTransaction,
} from "@solana/web3.js";
import { Account } from "../account";
import { Utxo } from "../utxo";
import * as anchor from "@coral-xyz/anchor";
import { Action, Transaction, TransactionParameters } from "../transaction";
import { sign } from "tweetnacl";
import * as splToken from "@solana/spl-token";
import { BN } from "@coral-xyz/anchor";
const circomlibjs = require("circomlibjs");

import {
  FEE_ASSET,
  UTXO_MERGE_MAXIMUM,
  UTXO_FEE_ASSET_MINIMUM,
  UTXO_MERGE_THRESHOLD,
  SIGN_MESSAGE,
  AUTHORITY,
  MERKLE_TREE_KEY,
  TOKEN_REGISTRY,
  merkleTreeProgramId,
  MINIMUM_LAMPORTS,
} from "../constants";
import { SolMerkleTree } from "../merkleTree/index";
import { VerifierZero } from "../verifiers/index";
import { Relayer } from "../relayer";
import { getUnspentUtxos } from "./buildBalance";
import { Provider } from "./provider";
import { getAccount, TOKEN_PROGRAM_ID } from "@solana/spl-token";
import { assert } from "chai";
import axios from "axios";
import { selectInUtxos } from "./selectInUtxos";
import { createOutUtxos, Recipient } from "./createOutUtxos";
import { strToArr } from "../utils";
import {
  CreateUtxoErrorCode,
  ProviderErrorCode,
  RelayerErrorCode,
  TransactionErrorCode,
  TransactionParametersErrorCode,
  UserError,
  UserErrorCode,
} from "../errors";
const message = new TextEncoder().encode(SIGN_MESSAGE);

type Balance = {
  symbol: string;
  amount: BN;
  tokenAccount: PublicKey;
  decimals: BN;
};

type TokenContext = {
  symbol: string;
  decimals: BN;
  tokenAccount: PublicKey;
  isNft: boolean;
  isSol: boolean;
};

export type CachedUserState = {
  utxos: Utxo[];
  seed: string;
};
export const convertAndComputeDecimals = (
  amount: BN | string | number,
  decimals: BN,
) => {
  return new BN(amount.toString()).mul(decimals);
};
var initLog = console.log;

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
  private seed?: string;
  constructor({
    provider,
    utxos = [],
    account = undefined,
  }: {
    provider: Provider;
    utxos?: Utxo[];
    account?: Account;
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
    this.account = account;
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
      let leavesPdas = await SolMerkleTree.getInsertedLeaves(
        MERKLE_TREE_KEY,
        this.provider.provider,
      );

      //TODO: add: "pending" to balances
      //TODO: add init by cached (subset of leavesPdas)
      const params = {
        leavesPdas,
        merkleTree: this.provider.solMerkleTree.merkleTree!,
        provider: this.provider.provider!,
        account: this.account,
        poseidon: this.provider.poseidon,
        merkleTreeProgram: merkleTreeProgramId,
      };
      const utxos = await getUnspentUtxos(params);

      this.utxos = utxos;
      console.log("✔️ updated utxos", this.utxos.length);
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

  async getRelayer(
    ataCreationFee: boolean = false,
  ): Promise<{ relayer: Relayer; feeRecipient: PublicKey }> {
    // TODO: pull an actually implemented relayer here via http request
    // This will then also remove the need to fund the relayer recipient account...
    let mockRelayer = new Relayer(
      this.provider.wallet!.publicKey,
      this.provider.lookUpTable!,
      SolanaKeypair.generate().publicKey,
      ataCreationFee ? new anchor.BN(500000) : new anchor.BN(100000),
    );
    await this.provider.provider!.connection.confirmTransaction(
      await this.provider.provider!.connection.requestAirdrop(
        mockRelayer.accounts.relayerRecipient,
        1_000_000,
      ),
      "confirmed",
    );
    const placeHolderAddress = SolanaKeypair.generate().publicKey;

    return { relayer: mockRelayer, feeRecipient: placeHolderAddress };
  }

  // TODO: in UI, support wallet switching, "prefill option with button"
  async getTxParams({
    tokenCtx,
    publicAmountSpl,
    publicAmountSol,
    action,
    userSplAccount = AUTHORITY,
    // for unshield
    recipientFee,
    recipientSPLAddress,
    // for transfer
    shieldedRecipients,
    ataCreationFee,
  }: {
    tokenCtx: TokenContext;
    publicAmountSpl?: BN;
    publicAmountSol?: BN;
    userSplAccount?: PublicKey;
    recipientFee?: PublicKey;
    recipientSPLAddress?: PublicKey;
    shieldedRecipients?: Recipient[];
    action: Action;
    ataCreationFee?: boolean;
  }): Promise<TransactionParameters> {
    var relayer;
    if (action === Action.TRANSFER || action === Action.UNSHIELD) {
      const { relayer: _relayer, feeRecipient: _feeRecipient } =
        await this.getRelayer(ataCreationFee);
      relayer = _relayer;
    }

    publicAmountSol = publicAmountSol ? publicAmountSol : new BN(0);
    publicAmountSpl = publicAmountSpl ? publicAmountSpl : new BN(0);

    if (action === Action.TRANSFER && !shieldedRecipients)
      throw new UserError(
        UserErrorCode.SHIELDED_RECIPIENT_UNDEFINED,
        "getTxParams",
        "Recipient not provided for transfer",
      );

    if (action !== Action.SHIELD && !relayer?.relayerFee)
      // TODO: could make easier to read by adding separate if/cases
      throw new UserError(
        RelayerErrorCode.RELAYER_FEE_UNDEFINED,
        "getTxParams",
        `No relayerFee provided for ${action.toLowerCase()}}`,
      );
    if (!this.account) {
      throw new UserError(
        CreateUtxoErrorCode.ACCOUNT_UNDEFINED,
        "getTxParams",
        "Account not defined",
      );
    }

    var inputUtxos: Utxo[] = [];
    var outputUtxos: Utxo[] = [];

    inputUtxos = selectInUtxos({
      publicMint: tokenCtx.tokenAccount,
      publicAmountSpl,
      publicAmountSol,
      recipients: shieldedRecipients,
      utxos: this.utxos,
      relayerFee: relayer?.relayerFee,
      action,
    });

    outputUtxos = createOutUtxos({
      publicMint: tokenCtx.tokenAccount,
      publicAmountSpl,
      inUtxos: inputUtxos,
      publicAmountSol, // TODO: add support for extra sol for unshield & transfer
      poseidon: this.provider.poseidon,
      relayerFee: relayer?.relayerFee,
      changeUtxoAccount: this.account,
      recipients: shieldedRecipients,
      action,
    });

    let txParams = new TransactionParameters({
      outputUtxos,
      inputUtxos,
      merkleTreePubkey: MERKLE_TREE_KEY,
      sender: action === Action.SHIELD ? userSplAccount : undefined,
      senderFee:
        action === Action.SHIELD ? this.provider.wallet!.publicKey : undefined,
      recipient: recipientSPLAddress,
      recipientFee,
      verifier: new VerifierZero(), // TODO: add support for 10in here -> verifier1
      poseidon: this.provider.poseidon,
      action,
      lookUpTable: this.provider.lookUpTable!,
      relayer,
    });
    return txParams;
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
  }: {
    token: string;
    recipient?: Account;
    publicAmountSpl?: number | BN | string;
    publicAmountSol?: number | BN | string;
    minimumLamports?: boolean;
    senderTokenAccount?: PublicKey;
  }) {
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
    if (recipient)
      throw new Error("Shields to other users not implemented yet!");
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
    let userSplAccount = undefined;
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

      let tokenBalance = await splToken.getAccount(
        this.provider.provider?.connection!,
        userSplAccount,
      );

      if (!tokenBalance)
        throw new UserError(
          UserErrorCode.ASSOCIATED_TOKEN_ACCOUNT_DOESNT_EXIST,
          "shield",
          "AssociatdTokenAccount doesn't exist!",
        );

      if (publicAmountSpl.gte(new BN(tokenBalance.amount.toString())))
        throw new UserError(
          UserErrorCode.INSUFFICIENT_BAlANCE,
          "shield",
          `Insufficient token balance! ${publicAmountSpl.toString()} bal: ${tokenBalance!
            .amount!}`,
        );

      try {
        const transaction = new createTransaction().add(
          splToken.createApproveInstruction(
            userSplAccount,
            AUTHORITY,
            this.provider.wallet!.publicKey,
            publicAmountSpl.toNumber(),
            [this.provider.wallet!.publicKey],
          ),
        );
        console.log({ transaction });

        const response = await this.provider.wallet!.sendAndConfirmTransaction(
          transaction,
        );
      } catch (e) {
        throw new UserError(
          UserErrorCode.APPROVE_ERROR,
          "shield",
          `Error approving token transfer! ${e}`,
        );
      }
    }

    const txParams = await this.getTxParams({
      tokenCtx,
      action: Action.SHIELD,
      publicAmountSol,
      publicAmountSpl,
      userSplAccount,
    });

    let tx = new Transaction({
      provider: this.provider,
      params: txParams,
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

    let response;
    if (!this.provider.wallet.node_wallet) {
      response = await axios.post("http://localhost:3331/updatemerkletree");
    }

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
  async unshield({
    token,
    publicAmountSpl,
    recipientSpl = new PublicKey(0),
    publicAmountSol,
    recipientSol,
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
    if (publicAmountSol && !recipientSol)
      throw new UserError(
        TransactionErrorCode.SOL_RECIPIENT_UNDEFINED,
        "getTxParams",
        "no recipient provided for sol unshield",
      );
    if (
      publicAmountSpl &&
      recipientSpl.toBase58() == new PublicKey(0).toBase58()
    )
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

    const txParams = await this.getTxParams({
      tokenCtx,
      publicAmountSpl: _publicSplAmount,
      action: Action.UNSHIELD,
      publicAmountSol: _publicSolAmount,
      recipientFee: recipientSol ? recipientSol : AUTHORITY,
      recipientSPLAddress: recipientSpl ? recipientSpl : undefined,
      ataCreationFee,
    });

    let tx = new Transaction({
      provider: this.provider,
      params: txParams,
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

    let response;
    if (!this.provider.wallet.node_wallet) {
      response = await axios.post("http://localhost:3331/updatemerkletree");
    }
    return { txHash, response };
  }

  // TODO: add separate lookup function for users.
  // TODO: add account parsing from and to string which is concat shielded pubkey and encryption key
  /**
   *
   * @param token mint
   * @param amount
   * @param recipient shieldedAddress (BN)
   * @param recipientEncryptionPublicKey (use strToArr)
   * @returns
   */
  async transfer({
    token,
    // alternatively we could use the recipient type here as well
    recipient,
    amountSpl,
    amountSol,
  }: {
    token: string;
    amountSpl?: BN | number | string;
    amountSol?: BN | number | string;
    recipient: Account;
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

    const txParams = await this.getTxParams({
      tokenCtx,
      action: Action.TRANSFER,
      shieldedRecipients: [
        {
          mint: tokenCtx.tokenAccount,
          account: recipient,
          solAmount: parsedSolAmount,
          splAmount: parsedSplAmount,
        },
      ],
    });
    let tx = new Transaction({
      provider: this.provider,
      params: txParams,
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
    let response;
    if (!this.provider.wallet.node_wallet) {
      response = await axios.post("http://localhost:3331/updatemerkletree");
    }
    return { txHash, response };
  }

  appInteraction() {
    throw new Error("not implemented yet");
  }
  /*
    *
    *return {
        inputUtxos,
        outputUtxos,
        txConfig: { in: number; out: number },
        verifier, can be verifier object
    }
    *
    */

  // TODO: consider removing payer property completely -> let user pass in the payer for 'load' and for 'shield' only.
  // TODO: evaluate whether we could use an offline instance of user, for example to generate a proof offline, also could use this to move error test to sdk
  /**
   *
   * @param cachedUser - optional cached user object
   * untested for browser wallet!
   */

  async load(cachedUser?: CachedUserState, provider?: Provider) {
    if (cachedUser) {
      this.seed = cachedUser.seed;
      this.utxos = cachedUser.utxos; // TODO: potentially add encr/decryption
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
    let leavesPdas = await SolMerkleTree.getInsertedLeaves(
      MERKLE_TREE_KEY,
      // @ts-ignore
      this.provider.provider,
    );

    const params = {
      leavesPdas,
      provider: this.provider.provider!,
      account: this.account,
      poseidon: this.provider.poseidon,
      merkleTreeProgram: merkleTreeProgramId,
      merkleTree: this.provider.solMerkleTree!.merkleTree!,
    };
    const utxos = await getUnspentUtxos(params);
    this.utxos = utxos;
  }

  // TODO: we need a non-anchor version of "provider" - (bundle functionality exposed by the wallet adapter into own provider-like class)
  /**
   *
   * @param provider - Light provider
   * @param cachedUser - Optional user state to instantiate from; e.g. if the seed is supplied, skips the log-in signature prompt.
   */

  static async load(
    provider: Provider,
    cachedUser?: CachedUserState,
  ): Promise<any> {
    try {
      const user = new User({ provider });
      await user.load(cachedUser, provider);
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
