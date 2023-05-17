import { expect, test } from "@oclif/test";
import { Keypair } from "@solana/web3.js";

// TODO: balance tests
// TODO: history tests
// TODO: change user tests

describe("Airdrop", () => {
  test
    .stdout()
    .command([
      "airdrop",
      "--token=SOL",
      `--amount=1000000000000000`,
      "ALA2cnz41Wa2v2EYUdkYHsg7VnKsbH1j7secM5aiP8k",
    ])
    .it(
      "airdrop 1000000000000000 sol to ALA2cnz41Wa2v2EYUdkYHsg7VnKsbH1j7secM5aiP8k",
      (ctx) => {
        console.log(ctx.stdout);
        expect(ctx.stdout).to.contain("Airdrop successful for user");
      }
    );

  test
    .stdout()
    .command([
      "airdrop",
      "--token=USDC",
      `--amount=1000000`,
      "ALA2cnz41Wa2v2EYUdkYHsg7VnKsbH1j7secM5aiP8k",
    ])
    .it(
      "airdrop 1000000 usdc to ALA2cnz41Wa2v2EYUdkYHsg7VnKsbH1j7secM5aiP8k",
      (ctx) => {
        console.log(ctx.stdout);
        expect(ctx.stdout).to.contain("Airdrop successful for user");
      }
    );
});

describe("Shield Sol", () => {
  test
    .stdout()
    .command(["shield", "--token=SOL", `--amountSol=2.7`])
    .it("Should shield 2.7 SOL", (ctx) => {
      console.log(ctx.stdout);
      expect(ctx.stdout).to.contain("Successfully shielded");
    });

  test
    .stdout()
    .command(["shield", "--token=SOL", `--amountSol=8.4`])
    .it("Should shield 8.4 SOL", (ctx) => {
      console.log(ctx.stdout);
      expect(ctx.stdout).to.contain("Successfully shielded");
    });

  test
    .stdout()
    .command(["balance"])
    .it("Should have balance of 11.1 SOL", (ctx) => {
      console.log(ctx.stdout);
      expect(ctx.stdout).to.contain("Total Sol Balance: 11100000000");
    });
});

describe("Unshield Sol", () => {
  let recipient = Keypair.generate().publicKey;

  test
    .stdout()
    .command([
      "unshield",
      "--token=SOL",
      "--amountSol=1.1",
      `--recipientSol=${recipient.toString()}`,
      `--recipientSpl=${recipient.toString()}`,
    ])
    .it("Unshield 1.1 SOL", (ctx) => {
      console.log(ctx.stdout);
      expect(ctx.stdout).to.contain("Successfully unshielded");
    });

  test
    .stdout()
    .command([
      "unshield",
      "--token=SOL",
      "--amountSol=0.2",
      `--recipientSol=${recipient.toString()}`,
      `--recipientSpl=${recipient.toString()}`,
    ])
    .it("Unshield 0.2 SOL", (ctx) => {
      console.log(ctx.stdout);
      expect(ctx.stdout).to.contain("Successfully unshielded");
    });
});

describe("Transfer Sol", () => {
  let recipient =
    "7zf5dv4sc7m2xskswD6J3CtoUoDApyGXRYodUtxTyPAXHmV121zqZR3aqBiL8SHPB4kxSFx12E9aiwmgtGWCjAT";
  test
    .stdout()
    .command(["transfer", "--token=SOL", "--amountSol=1.5", recipient])
    .it("Transfer 1.5 SOL", (ctx) => {
      console.log(ctx.stdout);
      expect(ctx.stdout).to.contain("Tokens successfully transferred");
    });
});

// add the usdc shield test
// add the udc unshield test
// add the usdc transfer test
// check balance test
// histiory tests
// fix usdc airdrop error in first iteration
