import {
  Enum,
  Keypair as SolanaKeypair,
  PublicKey,
  SystemProgram,
} from "@solana/web3.js";
import { Keypair } from "../keypair";
import { Utxo } from "utxo";
import * as anchor from "@coral-xyz/anchor";
import { LightInstance, Transaction, TransactionParameters } from "transaction";
import { sign } from "tweetnacl";
import * as splToken from "@solana/spl-token";

const circomlibjs = require("circomlibjs");
import {
  FEE_ASSET,
  UTXO_MERGE_MAXIMUM,
  UTXO_FEE_ASSET_MINIMUM,
  UTXO_MERGE_THRESHOLD,
  SIGN_MESSAGE,
  confirmConfig,
  AUTHORITY,
  MERKLE_TREE_KEY,
  TOKEN_REGISTRY,
  merkleTreeProgramId,
} from "../constants";
import {
  ADMIN_AUTH_KEYPAIR,
  initLookUpTableFromFile,
  recipientTokenAccount,
} from "test-utils";
import { SolMerkleTree } from "@merkleTree";
import { VerifierZero } from "verifiers";
import { Relayer } from "relayer";
import { getUnspentUtxos } from "./buildBalance";
const message = new TextEncoder().encode(SIGN_MESSAGE);

type Balance = {
  symbol: string;
  amount: number;
  tokenAccount: PublicKey;
  decimals: number;
};

type UtxoStatus = {
  symbol: string;
  tokenAccount: PublicKey;
  availableAmount: number; // max possible in 1 merge
  totalAmount: number;
  utxoCount: number;
};

type BrowserWallet = {
  signMessage: (message: Uint8Array) => Promise<Uint8Array>;
  signTransaction: (transaction: any) => Promise<any>;
  sendAndConfirmTransaction: (transaction: any) => Promise<any>;
  publicKey: PublicKey;
};

// TODO: Utxos should be assigned to a merkle tree
// TODO: evaluate optimization to store keypairs separately or store utxos in a map<Keypair, Utxo> to not store Keypairs repeatedly
// TODO: add support for wallet adapter (no access to payer keypair)
// TODO: one note:
/**
 *
 * @param browserWallet { signMessage, signTransaction, sendAndConfirmTransaction, publicKey}
 * @param payer Solana Keypair  - if not provided, browserWallet is used
 *
 */
export class User {
  payer?: SolanaKeypair;
  browserWallet?: BrowserWallet;
  keypair?: Keypair;
  utxos?: Utxo[];
  poseidon: any;
  lightInstance: LightInstance;
  private seed?: string;
  //TODO: rm need for payer in init-> load should suffice
  constructor({
    payer,
    browserWallet,
    utxos = [],
    keypair = undefined,
    poseidon = undefined,
    lightInstance,
  }: {
    payer?: SolanaKeypair;
    browserWallet?: BrowserWallet;
    utxos?: Utxo[];
    keypair?: Keypair;
    poseidon?: any;
    lightInstance: LightInstance;
  }) {
    if (!payer) {
      if (!browserWallet)
        throw new Error("No payer nor browserwallet provided");
      this.browserWallet = browserWallet;
    } else {
      this.payer = payer;
    }
    this.utxos = utxos;
    this.keypair = keypair;
    this.poseidon = poseidon;
    if (
      !lightInstance.lookUpTable ||
      !lightInstance.provider ||
      !lightInstance.solMerkleTree
    )
      throw new Error("LightInstance not properly initialized");
    this.lightInstance = lightInstance;
  }

  getBalance(): Balance[] {
    // TODO: merge amounts from utxo
    // TODO: consider if it makes sense to include a 'available balance' andor 'private balance'
    const balance = {
      symbol: "SOL",
      amount: 1e9,
      // unverifiedAmount: 1e8, // iterate here
      tokenAccount: SystemProgram.programId,
      decimals: 9,
    };
    return [balance];
  }

  // TODO: find clean way to support this (accepting/rejecting utxos, checking "available balance"),...
  async mergeUtxos(utxos: Utxo[]) {
    /** shielded transfer to self, merge 10-1; per asset (max: 5-1;5-1)
     * check *after* ACTION whether we can still merge in more.
     * TODO: add dust tagging protection (skip dust utxos)
     * Still torn - for regular devs this should be done automatically, e.g auto-prefacing any regular transaction.
     * whereas for those who want manual access there should be a fn to merge -> utxo = getutxosstatus() -> merge(utxos)
     */
  }
  getUtxoInbox() {
    // TODO: merge with getUtxoStatus?
    // returns all non-accepted utxos.
    // we'd like to enforce some kind of sanitary controls here.
    // would not be part of the main balance
  }
  getUtxoStatus(): UtxoStatus[] {
    let utxos: Utxo[] = []; // subset
    const status = {
      symbol: "SOL",
      tokenAccount: SystemProgram.programId,
      availableAmount: 1e9, // default: user has to approve incoming utxos -> instant merge.
      totalAmount: 2e9,
      utxoCount: 11,
    };
    return [status];
  }
  // getPrivacyScore() -> for unshields only, can separate into its own helper method
  // Fetch utxos should probably be a function such the user object is not occupied while fetching
  // but it would probably be more logical to fetch utxos here as well
  addUtxos() {}

  // TODO: v4: handle custom outCreator fns -> oututxo[] can be passed as param
  createOutUtxos({
    mint,
    amount,
    inUtxos,
  }: {
    mint: PublicKey;
    amount: number;
    inUtxos: Utxo[];
  }) {
    // amounts
    // TODO: get fee amounts, type of utxos,...
    if (!this.poseidon) throw new Error("Poseidon not initialized");
    if (!this.keypair) throw new Error("Shielded keypair not initialized");

    let u1 = new Utxo({
      poseidon: this.poseidon,
      assets: [],
      amounts: [],
      keypair: this.keypair, // if not self, use pubkey init
    });
    return [u1];
  }

  // TODO: adapt to rule: fee_asset is always first.
  // TODO: @Swen, add tests for hardcoded values
  // TODO: check if negative amounts need to separately be considered? (wd vs. deposit)
  // TODO: add pubAmount handling
  selectInUtxos({
    mint,
    privAmount,
    pubAmount,
  }: {
    mint: PublicKey;
    privAmount: number;
    pubAmount: number;
  }) {
    if (this.utxos === undefined) return [];
    if (this.utxos.length >= UTXO_MERGE_THRESHOLD)
      return [...this.utxos.slice(0, UTXO_MERGE_MAXIMUM)];
    if (this.utxos.length == 1) return [...this.utxos];

    const getAmount = (u: Utxo, asset: PublicKey) => {
      return u.amounts[u.assets.indexOf(asset)];
    };

    const getFeeSum = (utxos: Utxo[]) => {
      return utxos.reduce(
        (sum, utxo) => sum + getAmount(utxo, FEE_ASSET).toNumber(),
        0,
      );
    };

    var options: Utxo[] = [];

    const utxos = this.utxos.filter((utxo) => utxo.assets.includes(mint));

    var extraSolUtxos;
    if (mint !== FEE_ASSET) {
      extraSolUtxos = this.utxos
        .filter((utxo) => {
          let i = utxo.amounts.findIndex(
            (amount) => amount === new anchor.BN(0),
          );
          // The other asset must be 0 and SOL must be >0
          return (
            utxo.assets.includes(FEE_ASSET) &&
            i !== -1 &&
            utxo.amounts[utxo.assets.indexOf(FEE_ASSET)] > new anchor.BN(0)
          );
        })
        .sort(
          (a, b) =>
            getAmount(a, FEE_ASSET).toNumber() -
            getAmount(b, FEE_ASSET).toNumber(),
        );
    }
    console.log("extraSolUtxos: ", extraSolUtxos);

    // perfect match (2-in, 0-out)
    for (let i = 0; i < utxos.length; i++) {
      for (let j = 0; j < utxos.length; j++) {
        if (i == j || getFeeSum([utxos[i], utxos[j]]) < UTXO_FEE_ASSET_MINIMUM)
          continue;
        else if (
          getAmount(utxos[i], mint).add(getAmount(utxos[j], mint)) ==
          new anchor.BN(privAmount)
        ) {
          options.push(utxos[i], utxos[j]);
          return options;
        }
      }
    }

    // perfect match (1-in, 0-out)
    if (options.length < 1) {
      let match = utxos.filter(
        (utxo) => getAmount(utxo, mint) == new anchor.BN(privAmount),
      );
      if (match.length > 0) {
        const sufficientFeeAsset = match.filter(
          (utxo) => getFeeSum([utxo]) >= UTXO_FEE_ASSET_MINIMUM,
        );
        if (sufficientFeeAsset.length > 0) {
          options.push(sufficientFeeAsset[0]);
          return options;
        } else if (extraSolUtxos && extraSolUtxos.length > 0) {
          options.push(match[0]);
          /** handler 1: 2 in - 1 out here, with a feeutxo merged into place */
          /** TODO:  add as fallback: use another MINT utxo */
          // Find the smallest sol utxo that can cover the fee
          for (let i = 0; i < extraSolUtxos.length; i++) {
            if (
              getFeeSum([match[0], extraSolUtxos[i]]) >= UTXO_FEE_ASSET_MINIMUM
            ) {
              options.push(extraSolUtxos[i]);
              break;
            }
          }
          return options;
        }
      }
    }

    // 2 above amount - find the pair of the UTXO with the largest amount and the UTXO of the smallest amount, where its sum is greater than amount.
    if (options.length < 1) {
      for (let i = 0; i < utxos.length; i++) {
        for (let j = utxos.length - 1; j >= 0; j--) {
          if (
            i == j ||
            getFeeSum([utxos[i], utxos[j]]) < UTXO_FEE_ASSET_MINIMUM
          )
            continue;
          else if (
            getAmount(utxos[i], mint).add(getAmount(utxos[j], mint)) >
            new anchor.BN(privAmount)
          ) {
            options.push(utxos[i], utxos[j]);
            return options;
          }
        }
      }
    }

    // if 2-in is not sufficient to cover the transaction amount, use 10-in -> merge everything
    // cases where utxos.length > UTXO_MERGE_MAXIMUM are handled above already
    if (
      options.length < 1 &&
      utxos.reduce((a, b) => a + getAmount(b, mint).toNumber(), 0) >= privAmount
    ) {
      if (
        getFeeSum(utxos.slice(0, UTXO_MERGE_MAXIMUM)) >= UTXO_FEE_ASSET_MINIMUM
      ) {
        options = [...utxos.slice(0, UTXO_MERGE_MAXIMUM)];
      } else if (extraSolUtxos && extraSolUtxos.length > 0) {
        // get a utxo set of (...utxos.slice(0, UTXO_MERGE_MAXIMUM-1)) that can cover the amount.
        // skip last one to the left (smallest utxo!)
        for (let i = utxos.length - 1; i > 0; i--) {
          let sum = 0;
          let utxoSet = [];
          for (let j = i; j > 0; j--) {
            if (sum >= privAmount) break;
            sum += getAmount(utxos[j], mint).toNumber();
            utxoSet.push(utxos[j]);
          }
          if (
            sum >= privAmount &&
            getFeeSum(utxoSet) >= UTXO_FEE_ASSET_MINIMUM
          ) {
            options = utxoSet;
            break;
          }
        }
        // find the smallest sol utxo that can cover the fee
        if (options && options.length > 0)
          for (let i = 0; i < extraSolUtxos.length; i++) {
            if (
              getFeeSum([
                ...utxos.slice(0, UTXO_MERGE_MAXIMUM),
                extraSolUtxos[i],
              ]) >= UTXO_FEE_ASSET_MINIMUM
            ) {
              options = [...options, extraSolUtxos[i]];
              break;
            }
          }
        // TODO: add as fallback: use another MINT/third spl utxo
      }
    }

    return options;
  }

  // TODO: evaluate whether to move prepareUtxos here
  // maybe it makes sense since I might need new keypairs etc in this process
  // maybe not because we want to keep this class lean
  // TODO: shieldedkeypair/payer
  // TODO: in UI, support wallet switching, prefill option with button;
  async shield({
    token,
    amount,
    recipient,
  }: {
    token: string;
    amount: number;
    recipient?: anchor.BN; // TODO: consider replacing with Keypair.x type
  }) {
    // TODO: remove. is hardcoded (from env or smth)
    let tokenCtx =
      TOKEN_REGISTRY[TOKEN_REGISTRY.findIndex((t) => t.symbol === token)];

    if (!tokenCtx.isSol && this.payer) {
      try {
        await splToken.approve(
          this.lightInstance.provider!.connection,
          this.payer,
          tokenCtx.tokenAccount,
          AUTHORITY, //delegate
          this.payer, // owner
          amount * 2, // TODO: why is this *2?
          [this.payer],
        );
      } catch (error) {
        console.log(error);
      }
    } else {
      // TODO: implement browserWallet support for token.approve
    }

    let tx = new Transaction({
      instance: this.lightInstance,
      payer: ADMIN_AUTH_KEYPAIR,
      shuffleEnabled: false,
    });

    // TODO: add fees !
    let shieldUtxos = this.createOutUtxos({
      mint: tokenCtx.tokenAccount,
      amount,
      inUtxos: this.selectInUtxos({
        mint: tokenCtx.tokenAccount,
        privAmount: 0,
        pubAmount: amount,
      }),
    });

    let txParams = new TransactionParameters({
      outputUtxos: shieldUtxos,
      merkleTreePubkey: MERKLE_TREE_KEY,
      sender: tokenCtx.tokenAccount,
      senderFee: ADMIN_AUTH_KEYPAIR.publicKey,
      verifier: new VerifierZero(),
    });
    await tx.compileAndProve(txParams);

    try {
      let res = await tx.sendAndConfirmTransaction();
      console.log(res);
    } catch (e) {
      console.log(e);
      console.log("AUTHORITY: ", AUTHORITY.toBase58());
    }
    try {
      await tx.checkBalances();
    } catch (e) {
      console.log(e);
    }
  }

  /** unshield
   * @params token: string
   * @params amount: number - in base units (e.g. lamports for 'SOL')
   */
  async unshield({ token, amount }: { token: string; amount: number }) {
    // TODO: we could put these lines into a "init config" function

    const tokenCtx =
      TOKEN_REGISTRY[TOKEN_REGISTRY.findIndex((t) => t.symbol === token)];

    const inUtxos = this.selectInUtxos({
      mint: tokenCtx.tokenAccount,
      privAmount: 0,
      pubAmount: -amount,
    });
    const outUtxos = this.createOutUtxos({
      mint: tokenCtx.tokenAccount,
      amount,
      inUtxos,
    });

    // TODO: Create an actually implemented relayer here
    let relayer = new Relayer(
      ADMIN_AUTH_KEYPAIR.publicKey,
      this.lightInstance.lookUpTable!,
      SolanaKeypair.generate().publicKey,
      new anchor.BN(100000),
    );

    let tx = new Transaction({
      instance: this.lightInstance,
      relayer,
      payer: ADMIN_AUTH_KEYPAIR,
      shuffleEnabled: false,
    });

    //  TODO: replace with actually implemented accounts
    const origin = new anchor.web3.Account();
    var tokenRecipient = recipientTokenAccount;

    let txParams = new TransactionParameters({
      inputUtxos: inUtxos,
      outputUtxos: outUtxos,
      merkleTreePubkey: MERKLE_TREE_KEY,
      recipient: tokenRecipient,
      recipientFee: origin.publicKey,
      verifier: new VerifierZero(),
    });

    await tx.compileAndProve(txParams);

    // TODO: add check in client to avoid rent exemption issue
    // add enough funds such that rent exemption is ensured
    await this.lightInstance.provider!.connection.confirmTransaction(
      await this.lightInstance.provider!.connection.requestAirdrop(
        relayer.accounts.relayerRecipient,
        1_000_000,
      ),
      "confirmed",
    );
    try {
      let res = await tx.sendAndConfirmTransaction();
      console.log(res);
    } catch (e) {
      console.log(e);
      console.log("AUTHORITY: ", AUTHORITY.toBase58());
    }
  }

  async transfer({ token, amount }: { token: string; amount: number }) {
    // TODO: check for dry-er ways than to re-implement unshield

    const tokenCtx =
      TOKEN_REGISTRY[TOKEN_REGISTRY.findIndex((t) => t.symbol === token)];

    const inUtxos = this.selectInUtxos({
      mint: tokenCtx.tokenAccount,
      privAmount: amount,
      pubAmount: 0,
    });
    const outUtxos = this.createOutUtxos({
      mint: tokenCtx.tokenAccount,
      amount,
      inUtxos,
    });

    // TODO: Create an actually implemented relayer here
    let relayer = new Relayer(
      ADMIN_AUTH_KEYPAIR.publicKey,
      this.lightInstance.lookUpTable!,
      SolanaKeypair.generate().publicKey,
      new anchor.BN(100000),
    );

    let tx = new Transaction({
      instance: this.lightInstance,
      relayer,
      payer: ADMIN_AUTH_KEYPAIR,
      shuffleEnabled: false,
    });

    //  TODO: replace with actually implemented accounts
    const origin = new anchor.web3.Account();
    var tokenRecipient = recipientTokenAccount;

    let txParams = new TransactionParameters({
      inputUtxos: inUtxos,
      outputUtxos: outUtxos,
      merkleTreePubkey: MERKLE_TREE_KEY,
      recipient: tokenRecipient,
      recipientFee: origin.publicKey,
      verifier: new VerifierZero(),
    });

    await tx.compileAndProve(txParams);

    // TODO: add check in client to avoid rent exemption issue
    // add enough funds such that rent exemption is ensured
    await this.lightInstance.provider!.connection.confirmTransaction(
      await this.lightInstance.provider!.connection.requestAirdrop(
        relayer.accounts.relayerRecipient,
        1_000_000,
      ),
      "confirmed",
    );
    try {
      let res = await tx.sendAndConfirmTransaction();
      console.log(res);
    } catch (e) {
      console.log(e);
      console.log("AUTHORITY: ", AUTHORITY.toBase58());
    }
  }

  appInteraction() {}
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

  /** Sign, derive utxos, keypairs. Optionally, supply chached state to skip */
  // TODO: rm payer property -> let user pass in the payer for load and for shield only.
  async load(cachedUser?: User) {
    if (cachedUser) {
      this.keypair = cachedUser.keypair;
      this.seed = cachedUser.seed;
      this.payer = cachedUser.payer;
      this.browserWallet = cachedUser.browserWallet;
      this.poseidon = cachedUser.poseidon;
      this.utxos = cachedUser.utxos; // TODO: potentially add encr/decr
    }
    if (!this.seed) {
      if (this.payer) {
        const signature: Uint8Array = sign.detached(
          message,
          this.payer.secretKey,
        );
        this.seed = new anchor.BN(signature).toString();
      } else if (this.browserWallet) {
        const signature: Uint8Array = await this.browserWallet.signMessage(
          message,
        );
        this.seed = new anchor.BN(signature).toString();
      } else {
        throw new Error("No payer or browser wallet provided");
      }
    }
    if (!this.poseidon) {
      this.poseidon = await circomlibjs.buildPoseidonOpt();
    }
    if (!this.keypair) {
      this.keypair = new Keypair({
        poseidon: this.poseidon,
        seed: this.seed,
      });
    }

    let leavesPdas = await SolMerkleTree.getInsertedLeaves(MERKLE_TREE_KEY);

    const params = {
      leavesPdas,
      merkleTree: this.lightInstance.solMerkleTree!, // TODO: check diff solmt build vs constructor
      provider: this.lightInstance.provider,
      encryptionKeypair: this.keypair.encryptionKeypair, // TODO: save as keypair in class
      keypair: this.keypair,
      feeAsset: TOKEN_REGISTRY[0].tokenAccount,
      mint: TOKEN_REGISTRY[0].tokenAccount,
      poseidon: this.poseidon,
      merkleTreeProgram: merkleTreeProgramId,
    };
    const utxos = await getUnspentUtxos(params);
    // supply balance
    // TODO: debug getunspentutxos for this case dep.
    let balance = {
      [TOKEN_REGISTRY[0].symbol]: 0,
      [TOKEN_REGISTRY[1].symbol]: 0,
    };
  }
}
