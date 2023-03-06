import {
  Keypair as SolanaKeypair,
  PublicKey,
  SystemProgram,
} from "@solana/web3.js";
import { Account } from "../account";
import { Utxo } from "../utxo";
import * as anchor from "@coral-xyz/anchor";
import { Transaction, TransactionParameters } from "../transaction";
import { sign } from "tweetnacl";
import * as splToken from "@solana/spl-token";

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
} from "../constants";
import {
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

  async getBalance({ latest = true }: { latest?: boolean }): Promise<any> {
    const balances: Balance[] = [];
    if (!this.utxos) throw new Error("Utxos not initialized");
    if (!this.account) throw new Error("Keypair not initialized");
    if (!this.provider) throw new Error("Provider not initialized");
    if (!this.provider.poseidon) throw new Error("Poseidon not initialized");
    if (!this.provider.solMerkleTree)
      throw new Error("Merkle Tree not initialized");
    if (!this.provider.lookUpTable)
      throw new Error("Look up table not initialized");

    try {
      if (latest) {
        let leavesPdas = await SolMerkleTree.getInsertedLeaves(
          MERKLE_TREE_KEY,
          // @ts-ignore
          this.provider,
        );

        //TODO: add: "pending" to balances
        //TODO: add init by cached (subset of leavesPdas)
        // TODO: add incoming utxos
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
      return balances;
    } catch (err) {
      console.log("error in getting the user balance", { err });
      throw err;
    }
  }

  // TODO: v4: handle custom outCreator fns -> oututxo[] can be passed as param
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
    const { poseidon } = this.provider;
    if (!poseidon) throw new Error("Poseidon not initialized");
    if (!this.account) throw new Error("Shielded Account not initialized");

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
    let feeAsset = assets.find(
      (a) => a.asset.toBase58() === FEE_ASSET.toBase58(),
    );
    if (!feeAsset) throw new Error("Fee asset not found in assets");

    if (assets.length === 1) {
      // just fee asset as oututxo

      if (isTransfer) {
        let feeAssetSendUtxo = new Utxo({
          poseidon,
          assets: [assets[0].asset],
          amounts: [new anchor.BN(amount)],
          account: new Account({
            poseidon: poseidon,
            publicKey: recipient,
            encryptionPublicKey: recipientEncryptionPublicKey,
          }),
        });
        let feeAssetChangeUtxo = new Utxo({
          poseidon,
          assets: [assets[0].asset],
          amounts: [
            new anchor.BN(assets[0].amount)
              .sub(new anchor.BN(amount))
              .sub(relayer?.relayerFee || new anchor.BN(0)), // sub from change
          ], // rem transfer positive
          account: this.account,
        });

        return [feeAssetSendUtxo, feeAssetChangeUtxo];
      } else {
        let feeAssetChangeUtxo = new Utxo({
          poseidon,
          assets: [assets[0].asset],
          amounts: [new anchor.BN(assets[0].amount)],
          account: recipient
            ? new Account({
                poseidon: poseidon,
                publicKey: recipient,
                encryptionPublicKey: recipientEncryptionPublicKey,
              })
            : this.account, // if not self, use pubkey init
        });

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
            poseidon,
            assets: [assets[0].asset, asset.asset],
            amounts: [
              new anchor.BN(assets[0].amount),
              new anchor.BN(asset.amount),
            ],
            account: this.account, // if not self, use pubkey init // TODO: transfer: 1st is always recipient, 2nd change, both split sol min + rem to self
          });
          utxos.push(utxo1);
        } else {
          const utxo1 = new Utxo({
            poseidon,
            assets: [assets[0].asset, asset.asset],
            amounts: [new anchor.BN(0), new anchor.BN(asset.amount)],
            account: this.account, // if not self, use pubkey init
          });
          utxos.push(utxo1);
        }
      });
      if (utxos.length > 2)
        // TODO: implement for 3 assets (SPL,SPL,SOL)
        throw new Error(`Too many assets for outUtxo: ${assets.length}`);

      return utxos;
    }
  }

  // TODO: adapt to rule: fee_asset is always first.
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
    if (this.utxos === undefined) return [];
    if (this.utxos.length >= UTXO_MERGE_THRESHOLD)
      return [...this.utxos.slice(0, UTXO_MERGE_MAXIMUM)];
    if (this.utxos.length == 1) return [...this.utxos]; // TODO: check if this still works for spl...

    // TODO: turn these into static user.class methods
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
    } else console.log("mint is FEE_ASSET");

    console.log("extraSolUtxos (SPL!)", extraSolUtxos);
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
  getTxParams({
    tokenCtx,
    amount,
    action,
  }: {
    tokenCtx: TokenContext;
    amount: number;
    action: string;
  }): TransactionParameters {
    /// TODO: pass in flag "SHIELD", "UNSHIELD", "TRANSFER"
    // TODO: check with spl -> selects proper tokens?

    const inUtxos = this.selectInUtxos({
      mint: tokenCtx.tokenAccount,
      privAmount: 0,
      pubAmount: -1 * amount, // at shield, doesnt matter what val inutxos have.
    });
    // TODO: add fees !
    // console.log("inUtxos", inUtxos);
    let shieldUtxos = this.createOutUtxos({
      mint: tokenCtx.tokenAccount,
      amount,
      inUtxos,
    });
    // console.log("outUtxos", shieldUtxos);

    // TODO: get associated token account of nodewallet

    if (this.provider.nodeWallet) {
      let txParams = new TransactionParameters({
        outputUtxos: shieldUtxos,
        inputUtxos: inUtxos,
        merkleTreePubkey: MERKLE_TREE_KEY,
        sender: tokenCtx.isSol
          ? this.provider.nodeWallet!.publicKey
          : userTokenAccount, // TODO: must be users token account DYNAMIC here
        senderFee: this.provider.nodeWallet!.publicKey,
        verifier: new VerifierZero(), // TODO: add support for 10in here -> verifier1
      });
      return txParams;
    } else {
      const verifier = new VerifierZero(this.provider);
      let txParams = new TransactionParameters({
        outputUtxos: shieldUtxos,
        inputUtxos: inUtxos,
        merkleTreePubkey: MERKLE_TREE_KEY,
        sender: tokenCtx.isSol
          ? this.provider.browserWallet!.publicKey
          : userTokenAccount, // TODO: must be users token account DYNAMIC here
        senderFee: this.provider.browserWallet!.publicKey,
        verifier, // TODO: add support for 10in here -> verifier1
        provider: this.provider,
      });

      return txParams;

      // throw new Error("Browser wallet support not implemented yet!");
    }
  }
  /**
   *
   * @param amount e.g. 1 SOL = 1, 2 USDC = 2
   * @param token "SOL", "USDC", "USDT",
   * @param recipient optional, if not set, will shield to self
   */
  async shield({
    token,
    amount,
    recipient,
  }: {
    token: string;
    amount: number;
    recipient?: anchor.BN; // TODO: consider replacing with Keypair.x type
  }) {
    /**
     * TODO:
     * considerations for these.
     * - move relayer class to e.g. provider with optional override in tx?
     * - then would be good to add "tx and relayer" inception in there
     * - i wana hide the relayer. (default) -> new tx maybe too! shd be good.
     * - just important to know: where is the customization required where not
     *  */

    if (!this.provider) throw new Error("Provider not set!");
    if (recipient)
      throw new Error("Shields to other users not implemented yet!");
    // TODO: use getAssetByLookup fns instead (utils)
    // We're assuming "USDC for now"
    let tokenCtx = TOKEN_REGISTRY.find((t) => t.symbol === token);
    if (!tokenCtx) throw new Error("Token not supported!");

    console.log("tokenctx??", tokenCtx);
    if (!tokenCtx.isSol) {
      // throw new Error("SPL not supported yet!");
      // await createTestAccounts(this.provider.provider!.connection!);
      console.log("!tokenCtx.isSOL..");

      if (this.provider.nodeWallet) {
        // TODO: should be dynamic look up (getAssociatedTokenAccount,...)
        var balanceUserToken = null;
        try {
          console.log("userTokenAccount at shield", userTokenAccount);
          let balancet =
            await this.provider.provider!.connection.getTokenAccountBalance(
              // new PublicKey("CfyD2mSomGrjnyMKWrgNEk1ApaaUvKRDsnQngGkCVTFk"),
              userTokenAccount,
            );
          console.log("balance in shield..", balancet);

          balanceUserToken = await getAccount(
            this.provider!.provider!.connection,
            // hardcoded assumed for now
            userTokenAccount,
            "confirmed",
            TOKEN_PROGRAM_ID,
          );
        } catch (e) {
          console.log("couldn't find SPL account, initing...");
        }
        try {
          if (balanceUserToken == null) {
            console.log("newAccontWithTokens in shield..");
            await newAccountWithTokens({
              connection: this.provider!.provider!.connection,
              MINT: tokenCtx!.tokenAccount,
              ADMIN_AUTH_KEYPAIR: this.provider.nodeWallet!, // TODO: change naming
              userAccount: USER_TOKEN_ACCOUNT,
              amount: new anchor.BN(100_000_000_000),
            });
          }
        } catch (error) {
          console.log("spl token account creation failed: ", error);
        }
        amount = amount * tokenCtx.decimals;
        try {
          await splToken.approve(
            this.provider.provider!.connection,
            this.provider.nodeWallet!,
            userTokenAccount,
            // tokenCtx.tokenAccount, // TODO: must be user's token account
            // TODO: make dynamic based on verifier
            AUTHORITY, //delegate
            // this.provider.nodeWallet!, // owner
            USER_TOKEN_ACCOUNT, // owner2
            amount,
            [this.provider.nodeWallet!],
          );
        } catch (error) {
          console.log("error approving", error);
        }
      } else {
        // TODO: implement browserWallet support; for UI
        throw new Error("Browser wallet support not implemented yet!");
      }
    } else {
      console.log("- is SOL");
      amount = amount * tokenCtx.decimals;
    }
    // TODO: add browserWallet support
    let tx = new Transaction({
      provider: this.provider,
    });

    // add: different token select. -> tokenCtx
    const txParams = this.getTxParams({
      tokenCtx,
      amount,
      action: "SHIELD",
    });

    await tx.compileAndProve(txParams);

    try {
      let res = await tx.sendAndConfirmTransaction();
      console.log(
        `https://explorer.solana.com/tx/${res}?cluster=custom&customUrl=http%3A%2F%2Flocalhost%3A8899`,
      );
    } catch (e) {
      console.log({ e });
      // console.log("AUTHORITY: ", AUTHORITY.toBase58());
    }

    console.log = () => {};
    //@ts-ignore
    await tx.checkBalances(); // This is a test
    console.log = initLog;
    console.log("✔️ checkBalances success!");
    // TODO: replace this with a ping to a relayer that's running a merkletree update crank
    try {
      console.log("updating merkle tree...");
      await updateMerkleTreeForTest(
        this.provider.provider?.connection!,
        this.provider.browserWallet && this.provider,
      );
      console.log = initLog;
      console.log("✔️ updated merkle tree!");
    } catch (e) {
      console.log = initLog;
      console.log(e);
      throw new Error("Failed to update merkle tree!");
    }
  }

  /**
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
    const tokenCtx = TOKEN_REGISTRY.find((t) => t.symbol === token);
    if (!tokenCtx) throw new Error("Token not supported!");

    let recipientSPLAddress: PublicKey = new PublicKey(0);
    if (!tokenCtx.isSol) {
      throw new Error("SPL not implemented yet!");
      recipientSPLAddress = splToken.getAssociatedTokenAddressSync(
        tokenCtx!.tokenAccount,
        recipient,
      );
      // * units
    } else {
      amount = amount * 1e9;
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

    // TODO: replace with ping to relayer webserver

    let relayer = new Relayer(
      this.provider.browserWallet!
        ? this.provider.browserWallet.publicKey
        : this.provider.nodeWallet!.publicKey,
      this.provider.lookUpTable!,
      SolanaKeypair.generate().publicKey,
      new anchor.BN(100000),
    );

    /** payer is the nodeWallet of the relayer (always the one sending) */
    let tx = new Transaction({
      provider: this.provider,
    });

    const verifier = new VerifierZero(
      this.provider.browserWallet && this.provider,
    );

    // refactor idea: getTxparams -> in,out
    let txParams = new TransactionParameters({
      inputUtxos: inUtxos,
      outputUtxos: outUtxos,
      merkleTreePubkey: MERKLE_TREE_KEY,
      recipient: tokenCtx.isSol ? recipient : recipientSPLAddress, // TODO: check needs token account? // recipient of spl
      recipientFee: recipient, // feeRecipient
      verifier,
      relayer,
      provider: this.provider,
    });

    await tx.compileAndProve(txParams);

    // TODO: add check in client to avoid rent exemption issue
    // add enough funds such that rent exemption is ensured
    await this.provider.provider!.connection.confirmTransaction(
      await this.provider.provider!.connection.requestAirdrop(
        relayer.accounts.relayerRecipient,
        1_000_000,
      ),
      "confirmed",
    );
    try {
      let res = await tx.sendAndConfirmTransaction();
      console.log(
        `https://explorer.solana.com/tx/${res}?cluster=custom&customUrl=http%3A%2F%2Flocalhost%3A8899`,
      );
    } catch (e) {
      console.log(e);
      console.log("AUTHORITY: ", AUTHORITY.toBase58());
    }
    console.log = () => {};
    //@ts-ignore
    await tx.checkBalances();
    console.log = initLog;
    console.log("✔️ checkBalances success!");
    // TODO: replace this with a ping to a relayer that's running a crank
    try {
      console.log("updating merkle tree...");
      console.log = () => {};
      await updateMerkleTreeForTest(
        this.provider.provider?.connection!,
        this.provider.browserWallet && this.provider,
      );
      console.log = initLog;
      console.log("✔️ updated merkle tree!");
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
    // TODO: move this to a CLI command
    // THIS is for the current checkBalances...need privkey
    const randomShieldedKeypair = new Account({
      poseidon: this.provider.poseidon,
    });
    recipient = randomShieldedKeypair.pubkey;
    recipientEncryptionPublicKey =
      randomShieldedKeypair.encryptionKeypair.publicKey;
    // TEST END

    const tokenCtx = TOKEN_REGISTRY.find((t) => t.symbol === token);
    if (!tokenCtx) throw new Error("Token not supported!");

    // for is sol?
    if (!tokenCtx.isSol) throw new Error("SPL not implemented yet!");
    amount = amount * 1e9;
    // TODO: pull an actually implemented relayer here

    let relayer = new Relayer(
      this.provider.browserWallet!
        ? this.provider.browserWallet.publicKey
        : this.provider.nodeWallet!.publicKey,
      this.provider.lookUpTable!,
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

    let tx = new Transaction({
      provider: this.provider,
    });

    let randomRecipient = SolanaKeypair.generate().publicKey;
    console.log("randomRecipient", randomRecipient.toBase58());
    if (!tokenCtx.isSol) throw new Error("spl not implemented yet!");
    let txParams = new TransactionParameters({
      inputUtxos: inUtxos,
      outputUtxos: outUtxos,
      merkleTreePubkey: MERKLE_TREE_KEY,
      verifier: new VerifierZero(),
      recipient: randomRecipient,
      recipientFee: randomRecipient,
      relayer,
    });

    await tx.compileAndProve(txParams);
    // TODO: remove once relayer implemented.
    // add check in client to avoid rent exemption issue
    // add enough funds such that rent exemption is ensured
    await this.provider.provider!.connection.confirmTransaction(
      await this.provider.provider!.connection.requestAirdrop(
        relayer.accounts.relayerRecipient,
        100_000_000,
      ),
      "confirmed",
    );
    try {
      let res = await tx.sendAndConfirmTransaction();
      console.log(
        `https://explorer.solana.com/tx/${res}?cluster=custom&customUrl=http%3A%2F%2Flocalhost%3A8899`,
      );
    } catch (e) {
      console.log(e);
    }
    console.log = () => {};
    await tx.checkBalances(randomShieldedKeypair);
    console.log = initLog;

    console.log("✔️checkBalances success!");
    // TODO: replace this with a ping to a relayer that's running a crank
    try {
      console.log("updating merkle tree...");
      console.log = () => {};
      await updateMerkleTreeForTest(
        this.provider.provider?.connection!,
        this.provider.browserWallet && this.provider,
      );
      console.log = initLog;
      console.log("✔️updated merkle tree!");
    } catch (e) {
      console.log(e);
      throw new Error("Failed to update merkle tree!");
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
      // this.account = cachedUser.account;
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
    } catch (err) {
      console.log("while loading the user", { err });
      throw err;
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

export type CachedUserState = {
  utxos: Utxo[];
  seed: string;
};
