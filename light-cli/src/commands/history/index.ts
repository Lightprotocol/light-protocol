import { Command, Flags } from "@oclif/core";
import { IndexedTransaction } from "light-sdk";
import { getLoader, getUser } from "../../utils";

class TransactionHistoryCommand extends Command {
  static description = "Retrieve transaction history for the user";

  static flags = {
    latest: Flags.boolean({
      char: "l",
      description: "Retrieve the latest transaction history",
      default: true,
    }),
  };

  async run() {
    const { flags } = await this.parse(TransactionHistoryCommand);

    const { latest } = flags;

    const { loader, end } = getLoader("Retrieving use transaction histrory...");

    const user = await getUser();

    try {
      const transactions: IndexedTransaction[] =
        await user.getTransactionHistory(latest);

      // Log the transaction history
      transactions.forEach((transaction) => {
        console.log("--- Transaction ---");
        console.log("Block Time:", transaction.blockTime);
        console.log("Signer:", transaction.signer.toString());
        console.log("Signature:", transaction.signature);
        console.log("From:", transaction.from.toString());
        console.log("To:", transaction.to.toString());
        console.log(
          "Relayer Recipient Sol:",
          transaction.relayerRecipientSol.toString()
        );
        console.log("Type:", transaction.type);
        console.log(
          "Change Sol Amount:",
          transaction.changeSolAmount.toString()
        );
        console.log(
          "Public Amount Sol:",
          transaction.publicAmountSol.toString()
        );
        console.log(
          "Public Amount SPL:",
          transaction.publicAmountSpl.toString()
        );
        console.log("Relayer Fee:", transaction.relayerFee.toString());
        console.log("Message:", transaction.message);
        console.log("------------------");
      });

      end(loader);
    } catch (error) {
      end(loader);
      console.error("Error retrieving transaction history:", error);
    }
  }
}

TransactionHistoryCommand.examples = [
  "$ light history",
  "$ light history --latest=false",
];

module.exports = TransactionHistoryCommand;
