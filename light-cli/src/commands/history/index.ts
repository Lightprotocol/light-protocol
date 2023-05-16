import { Command, Flags } from "@oclif/core";
import { IndexedTransaction } from "@lightprotocol/zk.js";
import { CustomLoader, getUser } from "../../utils/utils";

class TransactionHistoryCommand extends Command {
  static description = "Retrieve transaction history for the user";

  static flags = {
    latest: Flags.boolean({
      char: "l",
      description: "Retrieve the latest transaction history",
      default: true,
    }),
  };

  protected finally(_: Error | undefined): Promise<any> {
    process.exit();
  }

  static examples: Command.Example[] = [
    "$ light history",
    "$ light history --latest=false",
  ];

  async run() {
    const { flags } = await this.parse(TransactionHistoryCommand);

    const { latest } = flags;

    const loader = new CustomLoader("Retrieving user transaction history...");

    loader.start();

    const user = await getUser();

    try {
      const transactions: IndexedTransaction[] =
        await user.getTransactionHistory(latest);

      // Log the transaction history
      transactions.forEach((transaction) => {
        this.log("\n--- Transaction ---");
        this.log("Block Time:", transaction.blockTime);
        this.log("Signer:", transaction.signer.toString());
        this.log("Signature:", transaction.signature);
        this.log("From:", transaction.from.toString());
        this.log("To:", transaction.to.toString());
        this.log(
          "Relayer Recipient Sol:",
          transaction.relayerRecipientSol.toString()
        );
        this.log("Type:", transaction.type);
        this.log("Change Sol Amount:", transaction.changeSolAmount.toString());
        this.log("Public Amount Sol:", transaction.publicAmountSol.toString());
        this.log("Public Amount SPL:", transaction.publicAmountSpl.toString());
        this.log("Relayer Fee:", transaction.relayerFee.toString());
        this.log("Message:", transaction.message);
        this.log("------------------");
      });

      loader.stop();
    } catch (error) {
      loader.stop();
      this.error(`\nError retrieving transaction history: ${error}`);
    }
  }
}

export default TransactionHistoryCommand;
