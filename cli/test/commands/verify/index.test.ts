import { expect, test } from "@oclif/test";
import { initTestEnvIfNeeded } from "../../../src/utils/initTestEnv";

describe("verify proof", () => {
  const correctProof: string =
    '{"ar":["0x2e58dac48029ecd8a5f2bc7dd1f8a3a7866e14cbd5f0763560893b201536f2b1","0xdc2d824a82967859f640c55d867303498a578a53d330374366ca55b9274c54d"],"bs":[["0x17cdea7a8ff87155793c981b8addc0d342b9ab4cd287db27756689a24cb98928","0x2ed89d16470679a7b2e03bc8b9a755fc94a0272dea04784789f751a5c7fd3a07"],["0x23c2d66d108196e0c6a246cc536d7e16e89440d25592532766c59742805ccae4","0x2ed33e32827e395076714e6035f1ee13b67ff92bc208595cb0fd061ebff1eaf7"]],"krs":["0x1e59b89a83220ee69e974a03d6026f88e21ba65bf2ec9f7cdc0d8a321df99e48","0x1a965b2d5645fd9f892415df1e9b1d1d4036291b7111cc86a3c36d4def54cbf9"]}';
  const invalidProof: string =
    '{"ar":["0x0","0xdc2d824a82967859f640c55d867303498a578a53d330374366ca55b9274c54d"],"bs":[["0x17cdea7a8ff87155793c981b8addc0d342b9ab4cd287db27756689a24cb98928","0x2ed89d16470679a7b2e03bc8b9a755fc94a0272dea04784789f751a5c7fd3a07"],["0x23c2d66d108196e0c6a246cc536d7e16e89440d25592532766c59742805ccae4","0x2ed33e32827e395076714e6035f1ee13b67ff92bc208595cb0fd061ebff1eaf7"]],"krs":["0x1e59b89a83220ee69e974a03d6026f88e21ba65bf2ec9f7cdc0d8a321df99e48","0x1a965b2d5645fd9f892415df1e9b1d1d4036291b7111cc86a3c36d4def54cbf9"]}';
  const roots =
    '"0x1ebf5c4eb04bf878b46937be63d12308bb14841813441f041812ea54ecb7b2d5"';
  const leafs =
    '"0x29176100eaa962bdc1fe6c654d6a3c130e96a4d1168b33848b897dc502820133"';

  test
    .stdout()
    .command([
      "verify",
      `--proof=${correctProof}`,
      `--roots=${roots}`,
      `--leafs=${leafs}`,
    ])
    .it("Verify proof with correct inputs", (ctx) => {
      expect(ctx.stdout).to.contain("Verified successfully");
    });

  test
    .stdout()
    .command([
      "verify",
      `--proof=${correctProof}`,
      `--roots="0x0"`,
      `--leafs=${leafs}`,
    ])
    .it("Verify proof with correct proof and invalid roots", (ctx) => {
      expect(ctx.stdout).to.contain("Verify failed");
    });

  test
    .stdout()
    .command([
      "verify",
      `--proof=${correctProof}`,
      `--roots="${roots}"`,
      `--leafs="0"`,
    ])
    .it("Verify proof with correct proof and invalid leafs", (ctx) => {
      expect(ctx.stdout).to.contain("Verify failed");
    });

  test
    .stdout()
    .command([
      "verify",
      `--proof=${invalidProof}`,
      `--roots="${roots}"`,
      `--leafs="0"`,
    ])
    .it("Verify proof with invalid proof and correct inputs", (ctx) => {
      expect(ctx.stdout).to.contain("Verify failed");
    });
});
