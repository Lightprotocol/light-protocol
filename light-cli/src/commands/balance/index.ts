import { Command, Flags } from "@oclif/core";
import {
  User,
  Balance,
  InboxBalance,
  Utxo,
  TOKEN_REGISTRY,
} from "@lightprotocol/zk.js";
import { CustomLoader, getUser } from "../../utils/utils";
import { PublicKey } from "@solana/web3.js";

class BalanceCommand extends Command {
  static description =
    "Retrieve the balance, inbox balance, or UTXOs for the user";

  static flags = {
    balance: Flags.boolean({
      char: "b",
      description: "Retrieve the balance",
      default: false,
    }),
    inbox: Flags.boolean({
      char: "i",
      description: "Retrieve the inbox balance",
      default: false,
    }),
    utxos: Flags.boolean({
      char: "u",
      description: "Retrieve the UTXOs",
      default: false,
    }),
    inboxUtxos: Flags.boolean({
      char: "x",
      description: "Retrieve the inbox UTXOs",
      default: false,
    }),
    latest: Flags.boolean({
      char: "l",
      description: "Retrieve the latest balance, inbox balance, or UTXOs",
      default: true,
    }),
    verbose: Flags.boolean({
      char: "v",
      description: "level of detailed output",
      default: false,
    }),
    token: Flags.string({
      char: "t",
      description: "The token to get balance of",
      default: "SOL",
    }),
  };

  protected finally(_: Error | undefined): Promise<any> {
    process.exit();
  }

  static examples = [
    "$ light balance --balance",
    "$ light balance --inbox",
    "$ light balance --utxos --inbox",
    "$ light balance --inboxUtxos",
    "$ light balance --latest=false",
  ];

  async run() {
    const { flags } = await this.parse(BalanceCommand);
    const { balance, inbox, utxos, latest, inboxUtxos, verbose, token } = flags;

    const loader = new CustomLoader("Retrieving balance...");

    loader.start();

    const user: User = await getUser();
    const tokenCtx = TOKEN_REGISTRY.get(token.toUpperCase());

    try {
      if (balance) {
        const result = await user.getBalance(latest);
        this.logBalance(result, verbose, tokenCtx!.mint);
      }
      if (inbox) {
        const result = await user.getUtxoInbox(latest);
        this.logBalance(result, verbose, tokenCtx!.mint);
      }
      if (utxos) {
        const result = await user.getAllUtxos();
        this.logUTXOs(result);
      }
      if (inboxUtxos) {
        const result = await user.getUtxoInbox();
        const utxos: Utxo[] = [];
        for (const iterator of result.tokenBalances.values()) {
          iterator.utxos.forEach((value) => {
            utxos.push(value);
          });
        }
        this.logUTXOs(utxos);
      }
      loader.stop();
    } catch (error) {
      loader.stop();
      this.error(`Error retrieving balance, inbox balance, or UTXOs: ${error}`);
    }
  }

  private logBalance(balance: Balance, verbose: boolean, token: PublicKey) {
    this.log("\n--- Balance ---");
    if (verbose) {
      this.log("Token Balances:", balance.tokenBalances);
      this.log("Program Balances:", balance.programBalances);
      this.log("NFT Balances:", balance.nftBalances);
      this.log("Transaction Nonce:", balance.transactionNonce);
      this.log(
        "Decryption Transaction Nonce:",
        balance.decryptionTransactionNonce
      );
      this.log(
        "Committed Transaction Nonce:",
        balance.committedTransactionNonce
      );
    }

    const tokenBalance = balance.tokenBalances.get(token.toString());

    if (tokenBalance && tokenBalance?.tokenData.symbol !== "SOL") {
      this.log(`${tokenBalance!.tokenData.symbol}:\n`);
      this.log(`total amount: ${tokenBalance!.totalBalanceSpl.toNumber()}`);
      this.log(`total sol amount: ${tokenBalance!.totalBalanceSol.toNumber()}`);
      this.log(`number of utxos:: ${tokenBalance!.utxos.size}`);
      this.log("\nUTXOS:");
      for (const iterator of tokenBalance?.utxos.values()!) {
        if (verbose) {
          console.log(iterator);
        }
        this.log(
          `usdc: ${iterator.amounts[1].toNumber()} sol: ${iterator.amounts[0].toNumber()} commitmentHash: ${
            iterator._commitment
          }`
        );
      }
    } else {
      this.log("Total Sol Balance:", balance.totalSolBalance.toString());
    }

    this.log("----------------");
  }

  private logUTXOs(utxos: Utxo[]) {
    this.log("\n--- UTXOs ---");
    for (const utxo of utxos) {
      this.log("UTXO:");
      this.log(`Amount: ${utxo.amounts}`);
      this.log(`Asset: ${utxo.assets}`);
      this.log(`Commitment: ${utxo._commitment}`);
      this.log(`Index: ${utxo.index}`);
    }
    this.log("----------------");
  }
}

module.exports = BalanceCommand;
