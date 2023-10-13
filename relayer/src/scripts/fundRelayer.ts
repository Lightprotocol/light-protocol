import { NETWORK, Network } from "../config";
import { fundRelayer } from "../setup/fundRelayer";

/// TODO: It might make sense to encourage manual funding of the relayer's accounts on all public Networks
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
  console.log("Funding relayer...");

  await fundRelayer();
  console.log("Relayer funded!");
})();
