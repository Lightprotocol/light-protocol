import { Args, Command, Flags } from "@oclif/core";

// TODO: add validations in the airdrop command
// TODO: add the validation in the config command
// TODO: test the wallet creation command
// TODO: add the spl airdrop command
// TODO: adapt the review oof swen and jorrit commits
// TODO: test the shield/ unshield
// TODO: have a transaction url log in the cli
// TODO: test the merkle tree commands
// TODO: balance logging and display
// TODO: review the jorrit commits on the balance
// TODO: test the transaction history command
// TODO: review and adapt the jorrit commits on the cli
// TODO: add the init , build in the cli that jorrit has created
// TODO: test the merge utxos and test the transfers after that
// TODO: test the utxos list command
// TODO: remove the hello and world command
// TODO: add the documentations
// TODO: add the testcases for the cli
// TODO: manage the user wallet from cli
// TODO: fix the force exit from the command

export default class Hello extends Command {
  static description = "Say hello";

  static examples = [
    `$ oex hello friend --from oclif
hello friend from oclif! (./src/commands/hello/index.ts)
`,
  ];
  static flags = {
    from: Flags.string({
      char: "f",
      description: "Who is saying hello",
      required: true,
    }),
  };

  static args = {
    person: Args.string({
      description: "Person to say hello to",
      required: true,
    }),
  };

  async run(): Promise<void> {
    const { args, flags } = await this.parse(Hello);

    this.log(
      `hello ${args.person} from ${flags.from}! (./src/commands/hello/index.ts)`
    );
  }
}
