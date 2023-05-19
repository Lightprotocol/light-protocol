import { Command, Flags, ux } from "@oclif/core";
import { BN } from "@coral-xyz/anchor";
import { IndexedTransaction } from "@lightprotocol/zk.js";
import { CustomLoader, getUser } from "../../utils/utils";

type TransactionHistory = {
  TransactionNumber: number;
  Timestamp: string;
  Signer: string;
  Signature: string;
  From: string;
  To: string;
  RelayerRecipientSOL: string;
  Type: string;
  PublicAmountSOL: number;
  PublicAmountSPL: number;
  RelayerFeeSOL: number;
}
class TransactionHistoryCommand extends Command {
  static description = "Show user transaction history";
  static flags = {
    'skip-fetch': Flags.boolean({
      char: "s",
      description: "Retrieve the latest transaction history: skip fetching from the indexer",
      parse: async () => false, 
      allowNo: true,
      hidden: true,
      default: true,
    }),
  };

  async run() {
    const { flags } = await this.parse(TransactionHistoryCommand);
    const latest = flags["skip-fetch"];

    const loader = new CustomLoader("Retrieving user transaction history...");
    loader.start();

    try {
      this.log('\n');

      const user = await getUser();
      
      const transactions: IndexedTransaction[] = await user.getTransactionHistory(false);

      transactions.reverse().forEach((transaction, index) => {
        let date = new Date(transaction.blockTime);
        let transactionHistory: TransactionHistory = {
          TransactionNumber: index,
          Timestamp: date.toString(),
          Type: `\x1b[32m${transaction.type}\x1b[0m`,
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
            this.logTransaction(transactionHistory, ["RelayerFee", "RelayerFeeSOL", "To", "RelayerRecipientSOL", "From"]);
            break;
          case "UNSHIELD":
            this.logTransaction(transactionHistory, ["From", "To", "RelayerFee"]);
            break;
          case "TRANSFER":
            this.logTransaction(transactionHistory, ["PublicAmountSOL", "PublicAmountSPL", "From", "To"]);
            break;
          default:
            this.logTransaction(transactionHistory); // If none of the cases match, it logs all keys and values
            break;
        }
      });
      loader.stop();
    } catch (error) {
      loader.stop();
      this.error(`\nFailed to retrieve transaction history!\n${error}`);
    }
  }

  private logTransaction(transaction: TransactionHistory, ignoreKeys: string[] = []): void {
    let tableData: any[] = [];
    let actionCheck = (transaction.Type == `\x1b[32mTRANSFER\x1b[0m` || transaction.Type == `\x1b[32mUNSHIELD\x1b[0m`);
    Object.keys(transaction).forEach(key => {
      if (!ignoreKeys.includes(key)) {
        // Transform the key from camel case to separate words, each starting with a capital letter.
        const formattedKey = key.replace(/([a-z0-9])([A-Z])/g, '$1 $2').replace(/([A-Z])([A-Z][a-z])/g, '$1 $2');
        let capitalizedKey = formattedKey.split(' ').map(word => word.charAt(0).toUpperCase() + word.slice(1)).join(' ');
        const value = transaction[key as keyof TransactionHistory];
        if (capitalizedKey === "Transaction Number") capitalizedKey = "Transaction Number   "
        if (capitalizedKey === "Signer" && actionCheck) {
          capitalizedKey = "Relayer Signer";
        }
        tableData.push({prop: `\x1b[34m${capitalizedKey}\x1b[0m`, value });
      }
    });
    ux.table(tableData, {
      prop: {header: ''},
      value: {header: ''},
    })
  }
  
  private convertToSol(amount: BN): number {
    const SOL_DECIMALS = new BN(1_000_000_000);
    return amount / SOL_DECIMALS
  }
}

export default TransactionHistoryCommand;
