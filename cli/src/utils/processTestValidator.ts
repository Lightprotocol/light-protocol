import { executeCommand, killProcessByName } from "./process";
import { sleep } from "@lightprotocol/stateless.js";
import { SOLANA_VALIDATOR_PROCESS_NAME } from "./constants";

export async function startTestValidator(solanaArgs: string[]) {
  const command = "solana-test-validator";
  await killProcessByName(SOLANA_VALIDATOR_PROCESS_NAME);
  await sleep(3000);
  console.log("Starting test validator...", command);
  await executeCommand({
    command,
    args: [...solanaArgs],
  });
}
