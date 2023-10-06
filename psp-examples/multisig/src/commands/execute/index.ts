import { Args, Command, Flags } from "@oclif/core";

export default class Execute extends Command {
  static description = "Execute transaction.";

  static examples = [
    `$ oex execute --transaction 1
Execute transaction 1.`,
  ];

  static flags = {
    transaction: Flags.string({
      char: "t",
      description: "Number of transaction to execute",
      required: true,
    }),
  };

  async run(): Promise<void> {
    const { flags } = await this.parse(Execute);
    this.log(`Execute transaction ${flags.transaction}.`);
  }
}
