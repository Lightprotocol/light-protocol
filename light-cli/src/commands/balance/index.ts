import { Command, Flags } from "@oclif/core";
import { User, Balance, InboxBalance, Utxo } from "light-sdk"; // Replace 'your-library' with the appropriate library name
import { getUser } from "../../utils";

class BalanceCommand extends Command {
  static description =
    "Retrieve the balance, inbox balance, or UTXOs for the user";

  static flags = {
    balance: Flags.boolean({
      char: "b",
      description: "Retrieve the balance",
      default: true,
      exclusive: ["inbox", "utxos"],
    }),
    inbox: Flags.boolean({
      char: "i",
      description: "Retrieve the inbox balance",
      default: false,
      exclusive: ["balance", "utxos"],
    }),
    utxos: Flags.boolean({
      char: "u",
      description: "Retrieve the UTXOs",
      default: false,
      exclusive: ["balance", "inbox"],
    }),
    latest: Flags.boolean({
      char: "l",
      description: "Retrieve the latest balance/inbox balance/UTXOs",
      default: true,
    }),
  };

  static examples = [
    "$ light balance",
    "$ light balance --inbox",
    "$ light balance --utxos",
    "$ light balance --latest=false",
  ];

  async run() {
    const { flags } = await this.parse(BalanceCommand);
    const { balance, inbox, utxos, latest } = flags;

    const user = await getUser();

    try {
      if (balance) {
        const result = await user.getBalance(latest);
        this.logBalance(result);
      } else if (inbox) {
        const result = await user.getUtxoInbox(latest);
        this.logInboxBalance(result);
      } else if (utxos) {
        const result = await user.getUtxos(latest);
        this.logUTXOs(result);
      }
    } catch (error) {
      this.error(`Error retrieving balance, inbox balance, or UTXOs ${error}`);
    }
  }

  private logBalance(balance: Balance) {
    this.log("--- Balance ---");
    this.log("Token Balances:", balance.tokenBalances);
    this.log("Program Balances:", balance.programBalances);
    this.log("NFT Balances:", balance.nftBalances);
    this.log("Transaction Nonce:", balance.transactionNonce);
    this.log(
      "Decryption Transaction Nonce:",
      balance.decryptionTransactionNonce
    );
    this.log("Committed Transaction Nonce:", balance.committedTransactionNonce);
    this.log("Total Sol Balance:", balance.totalSolBalance.toString());
    this.log("----------------");
  }

  private logInboxBalance(inboxBalance: InboxBalance) {
    this.log("--- Inbox Balance ---");
    this.log("Token Balances:", inboxBalance.tokenBalances);
    this.log("Program Balances:", inboxBalance.programBalances);
    this.log("NFT Balances:", inboxBalance.nftBalances);
    this.log("Transaction Nonce:", inboxBalance.transactionNonce);
    this.log(
      "Decryption Transaction Nonce:",
      inboxBalance.decryptionTransactionNonce
    );
    this.log(
      "Committed Transaction Nonce:",
      inboxBalance.committedTransactionNonce
    );
    this.log("Total Sol Balance:", inboxBalance.totalSolBalance.toString());
    this.log("Number of Inbox UTXOs:", inboxBalance.numberInboxUtxos);
    this.log("---------------------");
  }

  private logUTXOs(utxos: Utxo[]) {
    this.log("--- UTXOs ---");
    for (const utxo of utxos) {
      this.log("UTXO:");
      this.log(`  Amount: ${utxo.amounts}`);
      this.log(`  Asset: ${utxo.assets}`);
      this.log(`  Commitment: ${utxo._commitment}`);
      this.log(`  Index: ${utxo.index}`);
    }
    this.log("----------------");
  }
}

module.exports = BalanceCommand;
