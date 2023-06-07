import { Command, Flags, ux } from "@oclif/core";
import { IndexedTransaction } from "@lightprotocol/zk.js";
import { CustomLoader, getUser } from "../../utils/utils";
import { BN } from "@coral-xyz/anchor";

type TransactionHistory = {
  Timestamp: string,
  Signer: string,
  Signature: string,
  From: string,
  To: string,
  RelayerRecipientSol: string,
  Type: string,
  ChangeSolAmount: number,
  PublicAmountSol: number,
  PublicAmountSPL: number,
  RelayerFee: number
}
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
      /* transactions.forEach((transaction) => {
        this.log('\x1b[35m%s\x1b[0m', "\n--- Transaction ---");
        const date = new Date(transaction.blockTime);
        this.log('\x1b[34m%s\x1b[0m', "Block Time:", date.toString());
        this.log('\x1b[34m%s\x1b[0m', "Signer:", transaction.signer.toString());
        this.log('\x1b[34m%s\x1b[0m', "Signature:", transaction.signature);
        this.log('\x1b[34m%s\x1b[0m', "From:", transaction.from.toString());
        this.log('\x1b[34m%s\x1b[0m', "To:", transaction.to.toString());
        this.log('\x1b[34m%s\x1b[0m', 
          "Relayer Recipient Sol:",
          transaction.relayerRecipientSol.toString()
        );
        this.log('\x1b[34m%s\x1b[0m', "Type:", transaction.type);
        this.log('\x1b[34m%s\x1b[0m', "Change Sol Amount:", transaction.changeSolAmount.toString());
        this.log('\x1b[34m%s\x1b[0m', "Public Amount SOl:", transaction.publicAmountSol.toString());
        this.log('\x1b[34m%s\x1b[0m', "Public Amount SPL:", transaction.publicAmountSpl.toString());
        this.log('\x1b[34m%s\x1b[0m', "Relayer Fee:", transaction.relayerFee.toString());
        //this.log('\x1b[34m%s\x1b[0m', "Message:", transaction.message.toString('utf-8'));
        this.log("------------------");
      }); */

      transactions.reverse().forEach((transaction) => {
        let date = new Date(transaction.blockTime);
        let transactionHistory: TransactionHistory = {
          Timestamp: date.toString(),
          Signer: transaction.signer.toString(),
          Signature: transaction.signature,
          From: transaction.from.toString(),
          To: transaction.to.toString(),
          RelayerRecipientSol: transaction.relayerRecipientSol.toString(),
          Type: transaction.type,
          ChangeSolAmount: this.convertToSol(transaction.changeSolAmount),
          PublicAmountSol: this.convertToSol(transaction.publicAmountSol),
          PublicAmountSPL: transaction.publicAmountSpl.toNumber() / 100,
          RelayerFee: this.convertToSol(transaction.relayerFee)
        };

        switch (transaction.type) {
          case "SHIELD":
            this.logTransaction(transactionHistory, ["RelayerFee", "To"]);
            break;
          case "UNSHIELD":
            this.logTransaction(transactionHistory, ["From", "To", "RelayerFee"]);
            break;
          case "TRANSFER":
            this.logTransaction(transactionHistory, ["PublicAmountSol", "PublicAmountSPL", "ChangeSolAmount"]);
            break;
          default:
            this.logTransaction(transactionHistory); // If none of the cases match, it logs all keys and values
            break;
        }
        
      })
 
      loader.stop();
    } catch (error) {
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
    // console.log("------------------");
  }
  
  private convertToSol(amount: BN): number {
    const SOL_DECIMALS = new BN(1_000_000_000);
    return amount.div(SOL_DECIMALS).toNumber()
  }
  /* // Usage:
  let transaction: TransactionHistory = {
      BlockTime: "Sat Jun 03 2023 14:30:47 GMT+0100 (Western European Summer Time)",
      Signer: "ALA2cnz41Wa2v2EYUdkYHsg7VnKsbH1j7secM5aiP8k",
      Signature: "2iobSrDujicrAUSg4WVZiYZa4B9FeJSuhiAfzRpeCV1H7fJ2fTxmEghgCALKxbVqKdhnDyGGKu18LwimY7ee66mj",
      From: "ALA2cnz41Wa2v2EYUdkYHsg7VnKsbH1j7secM5aiP8k",
      To: "Eti4Rjkx7ow88XkaFbxRStmwadTp8p9J2nSv7NhtuqDU",
      RelayerRecipientSol: "11111111111111111111111111111111",
      Type: "SHIELD",
      ChangeSolAmount: 340000000,
      PublicAmountSol: 340000000,
      PublicAmountSPL: 0,
      RelayerFee: 0
  };
  
  logTransaction(transaction, "From", "To"); */
}

export default TransactionHistoryCommand;
