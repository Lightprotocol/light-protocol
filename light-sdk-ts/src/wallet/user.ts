import {
  Keypair as SolanaKeypair,
  PublicKey,
  SystemProgram,
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
import {
  ADMIN_AUTH_KEY,
  ADMIN_AUTH_KEYPAIR,
  createTestAccounts,
  MINT,
  newAccountWithTokens,
  updateMerkleTreeForTest,
  userTokenAccount,
  USER_TOKEN_ACCOUNT,
} from "../test-utils/index";
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
const message = new TextEncoder().encode(SIGN_MESSAGE);

type Balance = {
  symbol: string;
  amount: number;
  tokenAccount: PublicKey;
  decimals: number;
};

type TokenContext = {
  symbol: string;
  decimals: number;
  tokenAccount: PublicKey;
  isNft: boolean;
  isSol: boolean;
};

export type CachedUserState = {
  utxos: Utxo[];
  seed: string;
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
    if (!provider.browserWallet && !provider.nodeWallet)
      throw new Error("No wallet provided");

    if (!provider.lookUpTable || !provider.solMerkleTree || !provider.poseidon)
      throw new Error("Provider not properly initialized");

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
    if (!this.utxos) throw new Error("Utxos not initialized");
    if (!this.account) throw new Error("Keypair not initialized");
    if (!this.provider) throw new Error("Provider not initialized");
    if (!this.provider.poseidon) throw new Error("Poseidon not initialized");
    if (!this.provider.solMerkleTree)
      throw new Error("Merkle Tree not initialized");
    if (!this.provider.lookUpTable)
      throw new Error("Look up table not initialized");

    // try {
    if (latest) {
      let leavesPdas = await SolMerkleTree.getInsertedLeaves(
        MERKLE_TREE_KEY,
        this.provider.provider,
      );
      console.log("leaves pdas", leavesPdas.length);
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
        amount: 0,
        tokenAccount: token.tokenAccount,
        decimals: token.decimals,
      });
    });

    this.utxos.forEach((utxo) => {
      utxo.assets.forEach((asset, i) => {
        const tokenAccount = asset;
        const amount = utxo.amounts[i].toNumber();

        const existingBalance = balances.find(
          (balance) =>
            balance.tokenAccount.toBase58() === tokenAccount.toBase58(),
        );
        if (existingBalance) {
          existingBalance.amount += amount;
        } else {
          let tokenData = TOKEN_REGISTRY.find(
            (t) => t.tokenAccount.toBase58() === tokenAccount.toBase58(),
          );
          if (!tokenData)
            throw new Error(
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
    // } catch (err) {
    //   throw new Error(`Èrror in getting the user balance: ${err.message}`);
    // }
  }

  selectUtxos({
    publicMint,
    publicSplAmount,
    publicSolAmount,
    relayerFee,
    action,
    recipients,
  }: {
    publicMint: PublicKey;
    publicSplAmount?: BN;
    publicSolAmount?: BN;
    relayerFee?: BN;
    action: Action;
    recipients?: Recipient[];
  }): { inUtxos: Utxo[]; outUtxos: Utxo[] } {
    // selectInUtxos and createOutUtxos, return all utxos
    var inUtxos: Utxo[] = [];
    var outUtxos: Utxo[] = [];

    if (action === Action.TRANSFER && !recipients)
      throw new Error(
        "Recipient or RecipientEncryptionPublicKey not provided for transfer",
      );
    if (action !== Action.SHIELD && !relayerFee)
      // TODO: could make easier to read by adding separate if/cases
      throw new Error(`No relayerFee provided for ${action.toLowerCase()}}`);

    if (this.utxos) {
      if (action !== Action.TRANSFER) {
        inUtxos = selectInUtxos({
          mint: publicMint,
          extraSolAmount:
            publicMint.toBase58() === SystemProgram.programId.toBase58()
              ? 0
              : // @ts-ignore: quickfix will change in next pr with selectInUtxos refactor
                publicSolAmount.toNumber(),
          amount:
            publicMint.toBase58() === SystemProgram.programId.toBase58()
              ? // @ts-ignore: quickfix will change in next pr with selectInUtxos refactor
                publicSolAmount.toNumber()
              : // @ts-ignore: quickfix will change in next pr with selectInUtxos refactor
                publicSplAmount.toNumber(),
          utxos: this.utxos,
        });
      } else {
        inUtxos = selectInUtxos({
          mint: publicMint,
          extraSolAmount:
            // @ts-ignore: quickfix will change in next pr with selectInUtxos refactor
            recipients[0].solAmount.clone().toNumber(),
          amount:
            // @ts-ignore: quickfix will change in next pr with selectInUtxos refactor
            recipients[0].splAmount.clone().toNumber(),
          utxos: this.utxos,
        });
      }
    } else {
      inUtxos = [];
    }

    if (!this.account) {
      throw new Error("Account not defined");
    }
    outUtxos = createOutUtxos({
      publicMint,
      publicSplAmount,
      inUtxos,
      publicSolAmount, // TODO: add support for extra sol for unshield & transfer
      poseidon: this.provider.poseidon,
      relayerFee,
      changeUtxoAccount: this.account,
      recipients,
      action,
    });

    return { inUtxos, outUtxos };
  }

  async getRelayer(
    ataCreationFee: boolean = false,
  ): Promise<{ relayer: Relayer; feeRecipient: PublicKey }> {
    // TODO: pull an actually implemented relayer here via http request
    // This will then also remove the need to fund the relayer recipient account...
    let mockRelayer = new Relayer(
      this.provider.browserWallet!
        ? this.provider.browserWallet.publicKey
        : this.provider.nodeWallet!.publicKey,
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
    publicSplAmount,
    publicSolAmount,
    action,
    userSplAccount = AUTHORITY,
    // for unshield
    recipient,
    recipientSPLAddress,
    // for transfer
    shieldedRecipients,
  }: {
    tokenCtx: TokenContext;
    publicSplAmount?: BN;
    publicSolAmount?: BN;
    userSplAccount?: PublicKey;
    recipient?: PublicKey;
    recipientSPLAddress?: PublicKey;
    shieldedRecipients?: Recipient[];
    action: Action;
  }): Promise<TransactionParameters> {
    if (action === Action.SHIELD) {
      if (!publicSolAmount && !publicSplAmount)
        throw new Error(
          "No public amount provided. Shield needs a public amount.",
        );
      publicSolAmount = publicSolAmount ? publicSolAmount : new BN(0);
      publicSplAmount = publicSplAmount ? publicSplAmount : new BN(0);

      const { inUtxos, outUtxos } = this.selectUtxos({
        publicMint: tokenCtx.tokenAccount,
        publicSplAmount,
        publicSolAmount,
        action,
        recipients: shieldedRecipients,
      });

      if (this.provider.nodeWallet) {
        let txParams = new TransactionParameters({
          outputUtxos: outUtxos,
          inputUtxos: inUtxos,
          merkleTreePubkey: MERKLE_TREE_KEY,
          sender: tokenCtx.isSol
            ? this.provider.nodeWallet!.publicKey
            : userSplAccount, // TODO: must be users token account DYNAMIC here
          senderFee: this.provider.nodeWallet!.publicKey,
          verifier: new VerifierZero(), // TODO: add support for 10in here -> verifier1
          poseidon: this.provider.poseidon,
          action,
          lookUpTable: this.provider.lookUpTable!,
        });
        return txParams;
      } else {
        const verifier = new VerifierZero(this.provider);
        let txParams = new TransactionParameters({
          outputUtxos: outUtxos,
          inputUtxos: inUtxos,
          merkleTreePubkey: MERKLE_TREE_KEY,
          sender: tokenCtx.isSol
            ? this.provider.browserWallet!.publicKey
            : userSplAccount, // TODO: must be users token account DYNAMIC here
          senderFee: this.provider.browserWallet!.publicKey,
          verifier, // TODO: add support for 10in here -> verifier1
          provider: this.provider,
          poseidon: this.provider.poseidon,
          action,
          lookUpTable: this.provider.lookUpTable!,
        });

        return txParams;
      }
    } else if (action === Action.UNSHIELD) {
      if (!recipient) throw new Error("no recipient provided for unshield");

      let ataCreationFee = false;

      if (!tokenCtx.isSol) {
        if (!recipientSPLAddress)
          throw new Error("no recipient SPL address provided for unshield");
        let tokenBalance =
          await this.provider.connection?.getTokenAccountBalance(
            recipientSPLAddress,
          );
        if (!tokenBalance?.value.uiAmount) {
          /** Signal relayer to create the ATA and charge an extra fee for it */
          ataCreationFee = true;
        }
      }

      const { relayer, feeRecipient } = await this.getRelayer(ataCreationFee);
      const { inUtxos, outUtxos } = this.selectUtxos({
        publicMint: tokenCtx.tokenAccount,
        publicSplAmount,
        publicSolAmount,
        action,
        recipients: shieldedRecipients,
        relayerFee: relayer.relayerFee,
      });

      const verifier = new VerifierZero(
        this.provider.browserWallet && this.provider,
      );

      // refactor idea: getTxparams -> in,out
      let txParams = new TransactionParameters({
        inputUtxos: inUtxos,
        outputUtxos: outUtxos,
        merkleTreePubkey: MERKLE_TREE_KEY,
        recipient: recipientSPLAddress, // TODO: check needs token account? // recipient of spl
        recipientFee: recipient, // feeRecipient
        verifier,
        relayer,
        provider: this.provider,
        poseidon: this.provider.poseidon,
        action,
        lookUpTable: this.provider.lookUpTable,
      });
      return txParams;
    } else if (action === Action.TRANSFER) {
      if (!shieldedRecipients || shieldedRecipients.length === 0)
        throw new Error("no recipient provided for unshield");
      const { relayer, feeRecipient } = await this.getRelayer();

      const { inUtxos, outUtxos } = this.selectUtxos({
        publicMint: tokenCtx.tokenAccount,
        action,
        recipients: shieldedRecipients,
        relayerFee: relayer.relayerFee,
      });

      const verifier = new VerifierZero(
        this.provider.browserWallet && this.provider,
      );

      let txParams = new TransactionParameters({
        merkleTreePubkey: MERKLE_TREE_KEY,
        verifier,
        inputUtxos: inUtxos,
        outputUtxos: outUtxos,
        // recipient: feeRecipient,
        // recipientFee: feeRecipient,
        relayer,
        poseidon: this.provider.poseidon,
        action,
        lookUpTable: this.provider.lookUpTable,
      });
      return txParams;
    } else throw new Error("Invalid action");
  }

  /**
   *
   * @param amount e.g. 1 SOL = 1, 2 USDC = 2
   * @param token "SOL", "USDC", "USDT",
   * @param recipient optional, if not set, will shield to self
   * @param extraSolAmount optional, if set, will add extra SOL to the shielded amount
   * @param userTokenAccount optional, if set, will use this token account to shield from, else derives ATA
   */
  async shield({
    token,
    amount,
    recipient,
    extraSolAmount,
    userTokenAccount,
  }: {
    token: string;
    amount: number;
    recipient?: anchor.BN;
    extraSolAmount?: number;
    userTokenAccount?: PublicKey;
  }) {
    if (!this.provider) throw new Error("Provider not set!");
    if (recipient)
      throw new Error("Shields to other users not implemented yet!");
    let tokenCtx = TOKEN_REGISTRY.find((t) => t.symbol === token);
    if (!tokenCtx) throw new Error("Token not supported!");
    if (tokenCtx.isSol && userTokenAccount)
      throw new Error("Cannot use userTokenAccount for SOL!");
    let userSplAccount = null;
    if (!tokenCtx.isSol) {
      if (this.provider.nodeWallet) {
        if (userTokenAccount) {
          userSplAccount = userTokenAccount;
        } else {
          userSplAccount = splToken.getAssociatedTokenAddressSync(
            tokenCtx!.tokenAccount,
            this.provider!.nodeWallet!.publicKey,
          );
        }

        amount = amount * tokenCtx.decimals;

        let tokenBalance = await splToken.getAccount(
          this.provider.provider?.connection!,
          userSplAccount,
        );

        if (!tokenBalance) throw new Error("ATA doesn't exist!");

        if (amount >= tokenBalance.amount)
          throw new Error(
            `Insufficient token balance! ${amount} bal: ${tokenBalance!
              .amount!}`,
          );

        try {
          await splToken.approve(
            this.provider.provider!.connection,
            this.provider.nodeWallet!,
            userSplAccount, //userTokenAccount,
            AUTHORITY, //TODO: make dynamic based on verifier
            this.provider.nodeWallet!, //USER_TOKEN_ACCOUNT, // owner2
            amount,
            [this.provider.nodeWallet!],
          );
        } catch (e) {
          throw new Error(`Error approving token transfer! ${e}`);
        }
      } else {
        // TODO: implement browserWallet support; for UI
        throw new Error("Browser wallet support not implemented yet!");
      }
      extraSolAmount = extraSolAmount ? extraSolAmount * 1e9 : MINIMUM_LAMPORTS;
    } else {
      // amount = amount * tokenCtx.decimals;
      extraSolAmount = amount * tokenCtx.decimals;
      amount = 0;
    }

    const txParams = await this.getTxParams({
      tokenCtx,
      publicSplAmount: new BN(amount),
      action: Action.SHIELD,
      publicSolAmount: new BN(extraSolAmount),
      // @ts-ignore
      userSplAccount,
    });

    // TODO: add browserWallet support
    let tx = new Transaction({
      provider: this.provider,
      params: txParams,
    });

    await tx.compileAndProve();

    try {
      let res = await tx.sendAndConfirmTransaction();
      console.log(
        `https://explorer.solana.com/tx/${res}?cluster=custom&customUrl=http%3A%2F%2Flocalhost%3A8899`,
      );
    } catch (e) {
      throw new Error(`Error in tx.sendAndConfirmTransaction! ${e}`);
    }
    // console.log = () => {};

    await tx.checkBalances();
    console.log = initLog;
    console.log("✔️ checkBalances success!");
    if (this.provider.browserWallet) {
      const response = await axios.post(
        "http://localhost:3331/updatemerkletree",
      );
    }
  }

  /**
   * @params token: string
   * @params amount: number - in base units (e.g. lamports for 'SOL')
   * @params recipient: PublicKey - Solana address
   * @params extraSolAmount: number - optional, if not set, will use MINIMUM_LAMPORTS
   */
  async unshield({
    token,
    amount,
    recipient,
    extraSolAmount,
  }: {
    token: string;
    amount: number;
    recipient: PublicKey;
    extraSolAmount?: number;
  }) {
    const tokenCtx = TOKEN_REGISTRY.find((t) => t.symbol === token);
    if (!tokenCtx) throw new Error("Token not supported!");

    let recipientSPLAddress: PublicKey = new PublicKey(0);
    amount = amount * tokenCtx.decimals;

    if (!tokenCtx.isSol) {
      recipientSPLAddress = splToken.getAssociatedTokenAddressSync(
        tokenCtx!.tokenAccount,
        recipient,
      );

      extraSolAmount = extraSolAmount ? extraSolAmount * 1e9 : MINIMUM_LAMPORTS;
    } else {
      extraSolAmount = amount;
      amount = 0;
    }

    const txParams = await this.getTxParams({
      tokenCtx,
      publicSplAmount: new BN(amount),
      action: Action.UNSHIELD,
      publicSolAmount: new BN(extraSolAmount),
      recipient: tokenCtx.isSol ? recipient : recipientSPLAddress, // TODO: check needs token account? // recipient of spl
      recipientSPLAddress: recipientSPLAddress
        ? recipientSPLAddress
        : undefined,
    });

    let tx = new Transaction({
      provider: this.provider,
      params: txParams,
    });

    await tx.compileAndProve();
    try {
      let res = await tx.sendAndConfirmTransaction();
      console.log(
        `https://explorer.solana.com/tx/${res}?cluster=custom&customUrl=http%3A%2F%2Flocalhost%3A8899`,
      );
    } catch (e) {
      throw new Error(`Error in tx.sendAndConfirmTransaction! ${e}`);
    }
    // await tx.checkBalances();
    console.log("checkBalances INACTIVE");
    if (this.provider.browserWallet) {
      const response = await axios.post(
        "http://localhost:3331/updatemerkletree",
      );
    }
  }

  // TODO: add separate lookup function for users.
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
    amount,
    recipient, // shieldedaddress
    recipientEncryptionPublicKey,
    extraSolAmount,
  }: {
    token: string;
    amount: number;
    recipient: string;
    recipientEncryptionPublicKey: Uint8Array;
    extraSolAmount?: number;
  }) {
    const tokenCtx = TOKEN_REGISTRY.find((t) => t.symbol === token);
    if (!tokenCtx) throw new Error("This token is not supported!");

    amount = amount * tokenCtx.decimals;

    if (!tokenCtx.isSol) {
      extraSolAmount = extraSolAmount ? extraSolAmount * 1e9 : MINIMUM_LAMPORTS;
    } else {
      extraSolAmount = amount;
      amount = 0;
    }
    const _recipient: Uint8Array = strToArr(recipient.toString());

    const recipientAccount = Account.fromPubkey(
      _recipient,
      recipientEncryptionPublicKey,
      this.provider.poseidon,
    );
    const txParams = await this.getTxParams({
      tokenCtx,
      action: Action.TRANSFER,
      shieldedRecipients: [
        {
          mint: tokenCtx.tokenAccount,
          account: recipientAccount,
          solAmount: new BN(extraSolAmount.toString()),
          splAmount: new BN(amount.toString()),
        },
      ],
    });
    let tx = new Transaction({
      provider: this.provider,
      params: txParams,
    });

    await tx.compileAndProve();

    try {
      let res = await tx.sendAndConfirmTransaction();
      console.log(
        `https://explorer.solana.com/tx/${res}?cluster=custom&customUrl=http%3A%2F%2Flocalhost%3A8899`,
      );
    } catch (e) {
      throw new Error(`Error in tx.sendAndConfirmTransaction! ${e}`);
    }
    //@ts-ignore
    // await tx.checkBalances();
    if (this.provider.browserWallet) {
      const response = await axios.post(
        "http://localhost:3331/updatemerkletree",
      );
    }
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
  /**
   *
   * @param cachedUser - optional cached user object
   * untested for browser wallet!
   */

  async load(cachedUser?: CachedUserState, provider?: Provider) {
    if (cachedUser) {
      this.seed = cachedUser.seed;
      this.utxos = cachedUser.utxos; // TODO: potentially add encr/decryption
      if (!provider) throw new Error("No provider provided");
      this.provider = provider;
    }
    if (!this.seed) {
      if (this.provider.nodeWallet && this.provider?.browserWallet)
        throw new Error("Both payer and browser wallet are provided");
      if (this.provider.nodeWallet) {
        const signature: Uint8Array = sign.detached(
          message,
          this.provider.nodeWallet.secretKey,
        );
        this.seed = new anchor.BN(signature).toString();
      } else if (this.provider?.browserWallet) {
        const signature: Uint8Array =
          await this.provider.browserWallet.signMessage(message);
        this.seed = new anchor.BN(signature).toString();
      } else {
        throw new Error("No payer or browser wallet provided");
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
      this.provider.browserWallet && this.provider,
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
      throw new Error(`Error while loading user! ${e}`);
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
