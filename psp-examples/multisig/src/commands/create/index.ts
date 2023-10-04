import { Args, Command, Flags } from "@oclif/core";
import { MultiSigClient } from "../../client";
import { Multisig } from "../../multisig";

export default class Create extends Command {
  static description = "Create multisig.";

  static examples = [
    `$ oex create multisig --owners 1,2,3 (./src/commands/create/index.ts)
`,
  ];

  static flags = {
    signers: Flags.string({
      char: "s",
      description: "Comma separated array of signers",
      required: true,
    }),
  };

  // static args = {
  //   person: Args.string({
  //     description: "Person to say create multisig to",
  //     required: true,
  //   }),
  // };

  async run(): Promise<void> {
    const { flags } = await this.parse(Create);

    this.log(`Create multisig with signers: ${flags.signers}.`);

    const multisig = await Multisig.createMultiSig();
    await multisig.create();
    this.log(multisig.toString());
  }
}
