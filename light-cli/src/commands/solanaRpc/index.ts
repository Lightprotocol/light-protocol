import { Command, Flags } from '@oclif/core';
import { updateRpcEndpoint } from '../../utils'; // Assuming you have a file named 'utils.ts' exporting the 'updateRpcEndpoint' function

class UpdateRpcCommand extends Command {
  static description = 'Update the RPC endpoint of Solana';

  static flags = {
    rpcEndpoint: Flags.string({
      description: 'The new RPC endpoint to set',
      required: true,
    }),
  };

  async run() {
    const { flags } = this.parse(UpdateRpcCommand);

    const { rpcEndpoint } = flags;

    try {
      await updateRpcEndpoint(rpcEndpoint);

      this.log(`RPC endpoint updated to: ${rpcEndpoint}`);
    } catch (error) {
      this.error(`Failed to update RPC endpoint: ${error.message}`);
    }
  }
}

UpdateRpcCommand.examples = [
  '$ light-cli update-rpc --rpcEndpoint https://api.solana.com',
];

UpdateRpcCommand.strict = false;

export default UpdateRpcCommand;
