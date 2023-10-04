import { Args, Command, Flags } from "@oclif/core";

export default class Transaction extends Command {
  static description = "List of transactions.";

  static examples = [
    `$ oex transactions
List of transactions.`,
  ];

  async run(): Promise<void> {
    this.log(`Transactions.`);
  }
}
