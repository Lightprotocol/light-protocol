import { createVerfyingkeyRsFileArgv } from "../cli/src/utils/createRustVerifyingKey";

async function run() {
  await createVerfyingkeyRsFileArgv();
}

run().catch((e) => {
  throw new Error(e);
});
