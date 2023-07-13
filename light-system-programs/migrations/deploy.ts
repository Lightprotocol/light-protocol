// Migrations are an early feature. Currently, they're nothing more than this
// single deploy script that's invoked from the CLI, injecting a provider
// configured from the workspace's Anchor.toml.

import { createTestAccounts, setUpMerkleTree } from "@lightprotocol/zk.js";
import * as anchor from "@coral-xyz/anchor";
import { PublicKey } from "@solana/web3.js";

module.exports = async function (provider) {
  anchor.setProvider(provider);
  const newAuthority = new PublicKey(
    "CLEuMG7pzJX9xAuKCFzBP154uiG1GaNo4Fq7x6KAcAfG",
  );
  await createTestAccounts(provider.connection);
  await setUpMerkleTree(provider, newAuthority, true);
  process.exit();
};
