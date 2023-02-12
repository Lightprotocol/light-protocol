import {
  Enum,
  Keypair as SolanaKeypair,
  PublicKey,
  SystemProgram,
} from "@solana/web3.js";
import { Keypair } from "../keypair";
import { Utxo } from "../utxo";
import * as anchor from "@coral-xyz/anchor";
import {
  LightInstance,
  Transaction,
  TransactionParameters,
} from "../transaction";
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
  updateMerkleTreeForTest,
} from "../test-utils/index";
import { SolMerkleTree } from "../merkleTree/index";
import { VerifierZero } from "../verifiers/index";
import { Relayer } from "../relayer";
import { getUnspentUtxos } from "./buildBalance";
import { arrToStr, strToArr } from "../utils";
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

  async getBalance(): Promise<Balance[]> {
    const balances: Balance[] = [];
    if (!this.utxos) throw new Error("Utxos not initialized");
    if (!this.keypair) throw new Error("Keypair not initialized");
    if (!this.poseidon) throw new Error("Poseidon not initialized");
    if (!this.lightInstance) throw new Error("Light Instance not initialized");
    let leavesPdas = await SolMerkleTree.getInsertedLeaves(MERKLE_TREE_KEY);
    //TODO: add: "pending" to balances
    //TODO: add init by cached (subset of leavesPdas)
    // let leavesPdasU = await SolMerkleTree.getUninsertedLeaves(MERKLE_TREE_KEY);
    // console.log(
    //   "leavesPdas length?: ",
    //   leavesPdas.length,
    //   "uninserted length: ",
    //   leavesPdasU.length,
    // );
    const params = {
      leavesPdas,
      merkleTree: this.lightInstance.solMerkleTree!.merkleTree!,
      // TODO: check the diff between SolMerkleTree.build vs constructor
      provider: this.lightInstance.provider!,
      // encryptionKeypair: this.keypair.encryptionKeypair,
      keypair: this.keypair,
      // feeAsset: TOKEN_REGISTRY[0].tokenAccount,
      // mint: TOKEN_REGISTRY[0].tokenAccount, //
      poseidon: this.poseidon,
      merkleTreeProgram: merkleTreeProgramId,
    };
    const utxos = await getUnspentUtxos(params);
    this.utxos = utxos;

    console.log("latest inserted utxos:", this.utxos.length);
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
          // console.log("adding amount:", amount);
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
    return balances;
  }

  // TODO: v4: handle custom outCreator fns -> oututxo[] can be passed as param
  // ASSUMES: amount POS for shield
  createOutUtxos({
    mint,
    amount,
    inUtxos,
    recipient,
    recipientEncryptionPublicKey,
    relayer,
  }: {
    mint: PublicKey;
    amount: number;
    inUtxos: Utxo[];
    recipient?: anchor.BN;
    recipientEncryptionPublicKey?: Uint8Array;
    relayer?: Relayer;
  }) {
    console.log("inUtxos", inUtxos);
    console.log("createOutUtxos amount", amount);
    if (!this.poseidon) throw new Error("Poseidon not initialized");
    if (!this.keypair) throw new Error("Shielded keypair not initialized");

    // @unshield - Reject if insufficient inUtxos
    // TODO: check if better solution
    if (amount < 0) {
      let inAmount = 0;
      inUtxos.forEach((inUtxo) => {
        inUtxo.assets.forEach((asset, i) => {
          if (asset.toBase58() === mint.toBase58()) {
            inAmount += inUtxo.amounts[i].toNumber();
          }
        });
      });
      if (inAmount < Math.abs(amount)) {
        throw new Error(
          `Insufficient funds for unshield/transfer. In amount: ${inAmount}, out amount: ${amount}`,
        );
      }
    }
    var isTransfer = false;
    if (recipient && recipientEncryptionPublicKey && relayer) isTransfer = true;
    console.log(
      "istransfer?",
      recipient,
      recipientEncryptionPublicKey,
      isTransfer,
      relayer,
    );
    type Asset = { amount: number; asset: PublicKey };
    let assets: Asset[] = [];
    assets.push({
      asset: SystemProgram.programId,
      amount: 0,
    });
    /// For shields: add amount to asset for out
    // TODO: for spl-might want to consider merging 2-1 as outs .
    let assetIndex = assets.findIndex(
      (a) => a.asset.toBase58() === mint.toBase58(),
    );
    if (assetIndex === -1) {
      assets.push({ asset: mint, amount: !isTransfer ? amount : 0 });
    } else {
      assets[assetIndex].amount += !isTransfer ? amount : 0;
    }

    // add in-amounts to assets
    inUtxos.forEach((inUtxo) => {
      inUtxo.assets.forEach((asset, i) => {
        let assetAmount = inUtxo.amounts[i].toNumber();
        let assetIndex = assets.findIndex(
          (a) => a.asset.toBase58() === asset.toBase58(),
        );

        if (assetIndex === -1) {
          assets.push({ asset, amount: assetAmount });
        } else {
          assets[assetIndex].amount += assetAmount;
        }
      });
    });
    console.log("assets...", assets);
    let feeAsset = assets.find(
      (a) => a.asset.toBase58() === FEE_ASSET.toBase58(),
    );
    if (!feeAsset) throw new Error("Fee asset not found in assets");

    if (assets.length === 1) {
      // just fee asset as oututxo

      if (isTransfer) {
        let feeAssetSendUtxo = new Utxo({
          poseidon: this.poseidon,
          assets: [assets[0].asset],
          amounts: [new anchor.BN(amount)],
          keypair: new Keypair({
            poseidon: this.poseidon,
            publicKey: recipient,
            encryptionPublicKey: recipientEncryptionPublicKey,
          }),
        });
        let feeAssetChangeUtxo = new Utxo({
          poseidon: this.poseidon,
          assets: [assets[0].asset],
          amounts: [
            new anchor.BN(assets[0].amount)
              .sub(new anchor.BN(amount))
              .sub(relayer?.relayerFee || new anchor.BN(0)), // sub from change
          ], // rem transfer positive
          keypair: this.keypair,
        });
        console.log(
          "feeAssetSendUtxo amount: ",
          feeAssetSendUtxo.amounts[0].toNumber(),
        );
        console.log(
          "feeAssetChangeUtxo amount: ",
          feeAssetChangeUtxo.amounts[0].toNumber(),
        );

        return [feeAssetSendUtxo, feeAssetChangeUtxo];
      } else {
        let feeAssetChangeUtxo = new Utxo({
          poseidon: this.poseidon,
          assets: [assets[0].asset],
          amounts: [new anchor.BN(assets[0].amount)],
          keypair: recipient
            ? new Keypair({
                poseidon: this.poseidon,
                publicKey: recipient,
                encryptionPublicKey: recipientEncryptionPublicKey,
              })
            : this.keypair, // if not self, use pubkey init
        });
        console.log(
          "feeAssetUtxo amount: ",
          feeAssetChangeUtxo.amounts[0].toNumber(),
        );
        return [feeAssetChangeUtxo];
      }
    } else {
      // add for spl with transfer case.
      const utxos: Utxo[] = [];
      assets.slice(1).forEach((asset, i) => {
        // SPL: determine which is the sendUtxo and changeUtxo
        // TODO: also- split feeasset to cover min tx fee
        if (i === assets.length - 1) {
          // add feeasset as asset to the last spl utxo
          const utxo1 = new Utxo({
            poseidon: this.poseidon,
            assets: [assets[0].asset, asset.asset],
            amounts: [
              new anchor.BN(assets[0].amount),
              new anchor.BN(asset.amount),
            ],
            keypair: this.keypair, // if not self, use pubkey init // TODO: transfer: 1st is always recipient, 2nd change, both split sol min + rem to self
          });
          utxos.push(utxo1);
        } else {
          const utxo1 = new Utxo({
            poseidon: this.poseidon,
            assets: [assets[0].asset, asset.asset],
            amounts: [new anchor.BN(0), new anchor.BN(asset.amount)],
            keypair: this.keypair, // if not self, use pubkey init
          });
          utxos.push(utxo1);
        }
      });
      console.log("oututxos...:", utxos);
      if (utxos.length > 2)
        throw new Error(`Too many assets for oututxo: ${assets.length}`);

      return utxos;
    }
  }

  // TODO: adapt to rule: fee_asset is always first.
  // TODO: @Swen, add tests for hardcoded values
  // TODO: check if negative amounts need to separately be considered? (wd vs. deposit)
  selectInUtxos({
    mint,
    privAmount,
    pubAmount,
  }: {
    mint: PublicKey;
    privAmount: number;
    pubAmount: number;
  }) {
    const amount = privAmount + pubAmount; // TODO: verify that this is correct w -
    console.log("selectInUtxos amount", amount);
    if (this.utxos === undefined) return [];
    if (this.utxos.length >= UTXO_MERGE_THRESHOLD)
      return [...this.utxos.slice(0, UTXO_MERGE_MAXIMUM)];
    if (this.utxos.length == 1) return [...this.utxos]; // TODO: check if this still works for spl...

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

    console.log("utxos unfilted: ", this.utxos.length);
    const utxos = this.utxos.filter((utxo) => utxo.assets.includes(mint));
    console.log("utxos filtered (token): ", utxos.length);
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
    } else console.log("mint is FEE_ASSET");
    console.log("extraSolUtxos: ", extraSolUtxos);

    /**
     * for shields and transfers we'll always have spare utxos,
     * hence no reason to find perfect matches
     * */
    if (amount > 0) {
      // perfect match (2-in, 0-out)
      for (let i = 0; i < utxos.length; i++) {
        for (let j = 0; j < utxos.length; j++) {
          if (
            i == j ||
            getFeeSum([utxos[i], utxos[j]]) < UTXO_FEE_ASSET_MINIMUM
          )
            continue;
          else if (
            getAmount(utxos[i], mint).add(getAmount(utxos[j], mint)) ==
            new anchor.BN(amount)
          ) {
            options.push(utxos[i], utxos[j]);
            return options;
          }
        }
      }

      // perfect match (1-in, 0-out)
      if (options.length < 1) {
        let match = utxos.filter(
          (utxo) => getAmount(utxo, mint) == new anchor.BN(amount),
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
                getFeeSum([match[0], extraSolUtxos[i]]) >=
                UTXO_FEE_ASSET_MINIMUM
              ) {
                options.push(extraSolUtxos[i]);
                break;
              }
            }
            return options;
          }
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
            new anchor.BN(amount)
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
      utxos.reduce((a, b) => a + getAmount(b, mint).toNumber(), 0) >= amount
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
            if (sum >= amount) break;
            sum += getAmount(utxos[j], mint).toNumber();
            utxoSet.push(utxos[j]);
          }
          if (sum >= amount && getFeeSum(utxoSet) >= UTXO_FEE_ASSET_MINIMUM) {
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

  // TODO: in UI, support wallet switching, "prefill option with button"
  async shield({
    token,
    amount,
    recipient,
  }: {
    token: string;
    amount: number;
    recipient?: anchor.BN; // TODO: consider replacing with Keypair.x type
  }) {
    if (recipient)
      throw new Error("Shields to other users aren't not implemented yet!");
    if (!TOKEN_REGISTRY.find((t) => t.symbol === token))
      throw new Error("Token not supported!");
    console.log("TOKEN_REGISTRY:", TOKEN_REGISTRY);
    // TODO: use getAssetByLookup fns instead (utils)
    let tokenCtx =
      TOKEN_REGISTRY[TOKEN_REGISTRY.findIndex((t) => t.symbol === token)];
    console.log("AUTHORITY?", AUTHORITY.toBase58(), tokenCtx);
    if (!tokenCtx.isSol) {
      if (this.payer) {
        try {
          await splToken.approve(
            this.lightInstance.provider!.connection,
            this.payer,
            tokenCtx.tokenAccount, // TODO: must be user's token account
            AUTHORITY, //delegate
            this.payer, // owner
            amount * 1, // TODO: why is this *2? // was *2
            [this.payer],
          );
        } catch (error) {
          console.log("error approving", error);
        }
      } else {
        // TODO: implement browserWallet support; for UI
        throw new Error("Browser wallet support not implemented yet!");
      }
    } else {
      console.log("isSOL");
    }

    let tx = new Transaction({
      instance: this.lightInstance,
      payer: this.payer, // ADMIN_AUTH_KEYPAIR
      shuffleEnabled: false,
    });
    console.log("tx created!");
    const inUtxos = this.selectInUtxos({
      mint: tokenCtx.tokenAccount,
      privAmount: 0,
      pubAmount: -1 * amount, // at shield, doesnt matter what val inutxos have.
    });
    // TODO: add fees !
    let shieldUtxos = this.createOutUtxos({
      mint: tokenCtx.tokenAccount,
      amount,
      inUtxos,
    });
    console.log("outUtxos created!", shieldUtxos);

    let txParams = new TransactionParameters({
      outputUtxos: shieldUtxos,
      inputUtxos: inUtxos,
      merkleTreePubkey: MERKLE_TREE_KEY,
      sender: tokenCtx.isSol ? this.payer!.publicKey : tokenCtx.tokenAccount, // TODO: must be users token account
      senderFee: this.payer!.publicKey, //ADMIN_AUTH_KEYPAIR.publicKey, // feepayer??
      verifier: new VerifierZero(),
    });
    console.log("txParams created!");
    await tx.compileAndProve(txParams);
    console.log("tx compiled and proved!");

    try {
      let res = await tx.sendAndConfirmTransaction();
      console.log(res);
    } catch (e) {
      console.log(e);
      console.log("AUTHORITY: ", AUTHORITY.toBase58());
    }

    await tx.checkBalances();
    // TODO: replace this with a ping to a relayer that's running a crank
    try {
      await updateMerkleTreeForTest(this.lightInstance.provider!);
    } catch (e) {
      console.log(e);
      throw new Error("Failed to update merkle tree!");
    }
  }

  /** unshield
   * @params token: string
   * @params amount: number - in base units (e.g. lamports for 'SOL')
   * @params recipient: PublicKey - Solana address
   */
  async unshield({
    token,
    amount,
    recipient,
  }: {
    token: string;
    amount: number;
    recipient: PublicKey;
  }) {
    const tokenCtx =
      TOKEN_REGISTRY[TOKEN_REGISTRY.findIndex((t) => t.symbol === token)];

    let recipientSPLAddress: PublicKey = new PublicKey(0);
    if (!tokenCtx.isSol) {
      recipientSPLAddress = splToken.getAssociatedTokenAddressSync(
        tokenCtx.tokenAccount,
        recipient,
      );
      throw new Error("SPL not implemented yet!");
    }

    const inUtxos = this.selectInUtxos({
      mint: tokenCtx.tokenAccount,
      privAmount: 0,
      pubAmount: amount,
    });
    const outUtxos = this.createOutUtxos({
      mint: tokenCtx.tokenAccount,
      amount: -amount,
      inUtxos,
    });

    // TODO: Create an actually implemented relayer here
    let relayer = new Relayer(
      ADMIN_AUTH_KEYPAIR.publicKey,
      this.lightInstance.lookUpTable!,
      SolanaKeypair.generate().publicKey,
      new anchor.BN(200000),
    );

    let tx = new Transaction({
      instance: this.lightInstance,
      relayer,
      payer: ADMIN_AUTH_KEYPAIR,
      shuffleEnabled: false,
    });

    let txParams = new TransactionParameters({
      inputUtxos: inUtxos,
      outputUtxos: outUtxos,
      merkleTreePubkey: MERKLE_TREE_KEY,
      recipient: tokenCtx.isSol ? recipient : recipientSPLAddress, // TODO: check needs token account? // recipient of spl
      recipientFee: recipient, // recipient of sol
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
    } catch (e) {
      console.log(e);
      console.log("AUTHORITY: ", AUTHORITY.toBase58());
    }
    await tx.checkBalances();
    // TODO: replace this with a ping to a relayer that's running a crank
    try {
      await updateMerkleTreeForTest(this.lightInstance.provider!);
    } catch (e) {
      console.log(e);
      throw new Error("Failed to update merkle tree!");
    }
  }

  // TODO: add separate lookup function for users.
  // TODO: check for dry-er ways than to re-implement unshield. E.g. we could use the type of 'recipient' to determine whether to transfer or unshield.
  /**
   *
   * @param token mint
   * @param amount
   * @param recipient shieldedAddress (BN
   * @param recipientEncryptionPublicKey (use strToArr)
   * @returns
   */
  async transfer({
    token,
    amount,
    recipient, // shieldedaddress
    recipientEncryptionPublicKey,
  }: {
    token: string;
    amount: number;
    recipient: anchor.BN; // TODO: Keypair.pubkey -> type
    recipientEncryptionPublicKey: Uint8Array;
  }) {
    // console.log("this.keypair...", this.keypair?.encryptionKeypair);
    // return;
    // TEST CHECKBALANCES
    const randomShieldedKeypair = new Keypair({ poseidon: this.poseidon });
    recipient = randomShieldedKeypair.pubkey;
    recipientEncryptionPublicKey =
      randomShieldedKeypair.encryptionKeypair.publicKey;
    // TEST END

    const tokenCtx =
      TOKEN_REGISTRY[TOKEN_REGISTRY.findIndex((t) => t.symbol === token)];

    // TODO: pull an actually implemented relayer here
    const relayer = new Relayer(
      ADMIN_AUTH_KEYPAIR.publicKey,
      this.lightInstance.lookUpTable!,
      SolanaKeypair.generate().publicKey,
      new anchor.BN(100000),
    );
    const inUtxos = this.selectInUtxos({
      mint: tokenCtx.tokenAccount,
      privAmount: amount, // priv pub doesnt need to distinguish here
      pubAmount: 0,
    });
    const outUtxos = this.createOutUtxos({
      mint: tokenCtx.tokenAccount,
      amount: amount, // if recipient -> priv
      inUtxos,
      recipient: recipient,
      recipientEncryptionPublicKey: recipientEncryptionPublicKey,
      relayer: relayer,
    });
    // print encryptionKeypairs for outUtxos
    console.log(
      "OUT ENCKEYPAIRS: ",
      outUtxos.map((u) => u.keypair.encryptionKeypair),
    );
    // return;

    let tx = new Transaction({
      instance: this.lightInstance,
      relayer,
      payer: ADMIN_AUTH_KEYPAIR,
      shuffleEnabled: false,
    });

    let randomRecipient = new SolanaKeypair().publicKey;
    console.log("random recipient: ", randomRecipient.toBase58());
    if (!tokenCtx.isSol) throw new Error("spl not implemented yet!");
    // TODO: why have to pass in a recipient?
    let txParams = new TransactionParameters({
      inputUtxos: inUtxos,
      outputUtxos: outUtxos,
      merkleTreePubkey: MERKLE_TREE_KEY,
      verifier: new VerifierZero(),
      recipient: randomRecipient,
      recipientFee: randomRecipient,
    });

    await tx.compileAndProve(txParams);
    // TODO: add check in client to avoid rent exemption issue
    // add enough funds such that rent exemption is ensured
    await this.lightInstance.provider!.connection.confirmTransaction(
      await this.lightInstance.provider!.connection.requestAirdrop(
        relayer.accounts.relayerRecipient,
        100_000_000,
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
    await tx.checkBalances(randomShieldedKeypair);
    console.log("checked balance successfully!");
    // TODO: replace this with a ping to a relayer that's running a crank
    try {
      await updateMerkleTreeForTest(this.lightInstance.provider!);
    } catch (e) {
      console.log(e);
      throw new Error("Failed to update merkle tree!");
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

  // TODO: consider removing payer property completely -> let user pass in the payer for 'load' and for 'shield' only.
  async load(cachedUser?: User) {
    if (cachedUser) {
      this.keypair = cachedUser.keypair;
      this.seed = cachedUser.seed;
      this.payer = cachedUser.payer;
      this.browserWallet = cachedUser.browserWallet;
      this.poseidon = cachedUser.poseidon;
      this.utxos = cachedUser.utxos; // TODO: potentially add encr/decryption
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

    /** TEMP: ensure leaves are inserted. TODO: Move to relayer */
    let leavesPdas = await SolMerkleTree.getInsertedLeaves(MERKLE_TREE_KEY);
    let uninsertedLeaves = await SolMerkleTree.getUninsertedLeaves(
      MERKLE_TREE_KEY,
    );
    console.log("user.load - leavesPdas inserted length: ", leavesPdas.length);
    console.log(
      "user.load - leavesPdas uninserted length: ",
      uninsertedLeaves.length,
    );
    if (uninsertedLeaves.length > 0) {
      try {
        await updateMerkleTreeForTest(this.lightInstance.provider!);
        leavesPdas = await SolMerkleTree.getInsertedLeaves(MERKLE_TREE_KEY);
      } catch (e) {
        console.log("user.load - couldn't update merkletree: ", e);
      }
    }
    const params = {
      leavesPdas,
      provider: this.lightInstance.provider!,
      keypair: this.keypair,
      poseidon: this.poseidon,
      merkleTreeProgram: merkleTreeProgramId,
      merkleTree: this.lightInstance.solMerkleTree!.merkleTree!,
    };
    const utxos = await getUnspentUtxos(params);
    this.utxos = utxos;
  }

  // TODO: find clean way to support this (accepting/rejecting utxos, checking "available balance"),...
  /** shielded transfer to self, merge 10-1; per asset (max: 5-1;5-1)
   * check *after* ACTION whether we can still merge in more.
   * TODO: add dust tagging protection (skip dust utxos)
   * Still torn - for regular devs this should be done automatically, e.g auto-prefacing any regular transaction.
   * whereas for those who want manual access there should be a fn to merge -> utxo = getutxosstatus() -> merge(utxos)
   */
  async mergeUtxos(utxos: Utxo[]) {}
  // TODO: merge with getUtxoStatus?
  // returns all non-accepted utxos.
  // we'd like to enforce some kind of sanitary controls here.
  // would not be part of the main balance
  getUtxoInbox() {}
  getUtxoStatus() {}
  // getPrivacyScore() -> for unshields only, can separate into its own helper method
  // Fetch utxos should probably be a function such the user object is not occupied while fetching
  // but it would probably be more logical to fetch utxos here as well
  addUtxos() {}
}
