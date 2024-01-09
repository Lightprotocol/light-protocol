import { NETWORK, Network } from "../config";
import { fundRpc } from "../setup/fundRpc";

/// TODO: It might make sense to encourage manual funding of the rpc's accounts on all public Networks
(async () => {
  if (NETWORK === Network.MAINNET)
    throw new Error(
      "Don't run this on mainnet. Fund your account keys manually instead",
    );
  if (NETWORK === Network.LOCALNET)
    throw new Error(
      "Don't run this on localnet. Run light test-validator instead.",
    );
  NETWORK === Network.TESTNET
    ? console.log("Running on testnet")
    : console.log("Running on devnet");
  console.log("Funding rpc...");

  await fundRpc();
  console.log("Rpc funded!");
})();
