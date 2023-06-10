import { Command, Flags } from "@oclif/core";
import { BN } from "@coral-xyz/anchor";
import { IndexedTransaction } from "@lightprotocol/zk.js";
import { CustomLoader, getUser } from "../../utils/utils";

type TransactionHistory = {
  Timestamp: string,
  Signer: string,
  Signature: string,
  From: string,
  To: string,
  RelayerRecipientSOL: string,
  Type: string,
  PublicAmountSOL: number,
  PublicAmountSPL: number,
  RelayerFeeSOL: number
}
class TransactionHistoryCommand extends Command {
  static description = "Retrieve transaction history for the user";

/*   static flags = {
    'skip-fetch': Flags.boolean({
      char: "s",
      description: "Retrieve the latest transaction history: skip fetching from the indexer",
      default: true,
      parse: async () => false, 
    }),
  }; */

  protected finally(_: Error | undefined): Promise<any> {
    process.exit();
  }

  static examples: Command.Example[] = [
    "$ light history",
    "$ light history --skip-fetch",
  ];

  async run() {
    const { flags } = await this.parse(TransactionHistoryCommand);
    const latest = flags["skip-fetch"];

    const loader = new CustomLoader("Retrieving user transaction history...");
    loader.start();

    const user = await getUser();

    try {
      const transactions: IndexedTransaction[] =
        await user.getTransactionHistory();

      transactions.reverse().forEach((transaction) => {
        let date = new Date(transaction.blockTime);
        let transactionHistory: TransactionHistory = {
          Timestamp: date.toString(),
          Type: transaction.type,
          PublicAmountSOL: this.convertToSol(transaction.publicAmountSol),
          PublicAmountSPL: transaction.publicAmountSpl / 100,
          From: transaction.from.toString(),
          To: transaction.to.toString(),
          RelayerRecipientSOL: transaction.relayerRecipientSol.toString(),
          RelayerFeeSOL: this.convertToSol(transaction.relayerFee),
          Signer: transaction.signer.toString(),
          Signature: transaction.signature,

        };
        switch (transaction.type) {
          case "SHIELD":
            this.logTransaction(transactionHistory, ["RelayerFee", "To", "RelayerRecipientSOL"]);
            break;
          case "UNSHIELD":
            this.logTransaction(transactionHistory, ["From", "To", "RelayerFee"]);
            break;
          case "TRANSFER":
            this.logTransaction(transactionHistory, ["PublicAmountSOL", "PublicAmountSPL", "ChangeSolAmount", "From", "To"]);
            break;
          default:
            this.logTransaction(transactionHistory); // If none of the cases match, it logs all keys and values
            break;
        }
      });
      loader.stop();
    } catch (error) {
      this.warn(error as Error);
      loader.stop();
      this.error(`\nError retrieving transaction history: ${error}`);
    }
  }

  private logTransaction(transaction: TransactionHistory, ignoreKeys: string[] = []): void {
    this.log('\x1b[35m%s\x1b[0m', "\n--- Transaction ---");
    Object.keys(transaction).forEach(key => {
      if (!ignoreKeys.includes(key)) {
        // Transform the key from camel case to separate words, each starting with a capital letter.
        const formattedKey = key.replace(/([a-z0-9])([A-Z])/g, '$1 $2').replace(/([A-Z])([A-Z][a-z])/g, '$1 $2');
        const capitalizedKey = formattedKey.split(' ').map(word => word.charAt(0).toUpperCase() + word.slice(1)).join(' ');
        this.log(`\x1b[34m${capitalizedKey}\x1b[0m: ${transaction[key as keyof TransactionHistory]}`);
      }
    });
  }
  
  private convertToSol(amount: BN): number {
    const SOL_DECIMALS = new BN(1_000_000_000);
    return amount / SOL_DECIMALS
  }
}

export default TransactionHistoryCommand;
