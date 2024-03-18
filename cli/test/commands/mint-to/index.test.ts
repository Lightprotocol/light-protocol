import { expect, test } from "@oclif/test";
import { initTestEnvIfNeeded } from "../../../src/utils/initTestEnv";
import { getPayer, getSolanaRpcUrl } from "../../../src";
import { Connection } from "@solana/web3.js";
import { createMint } from "@lightprotocol/compressed-token";
import { byteArrayToKeypair, confirmTx } from "@lightprotocol/stateless.js";
import { requestAirdrop } from "../../helpers/helpers";

describe("mint-to", () => {
  const FIXED_BOB = byteArrayToKeypair([
    23, 72, 199, 170, 152, 40, 30, 187, 91, 132, 88, 170, 94, 32, 89, 164, 164,
    38, 123, 3, 79, 17, 23, 83, 112, 91, 160, 140, 116, 9, 99, 38, 217, 144, 62,
    153, 200, 117, 213, 6, 62, 39, 186, 56, 34, 149, 58, 188, 99, 182, 87, 74,
    84, 182, 157, 45, 133, 253, 230, 193, 176, 160, 72, 249,
  ]);

  const MINT_AUTHORITY = byteArrayToKeypair([
    122, 239, 192, 18, 21, 29, 237, 120, 104, 95, 247, 150, 181, 218, 207, 60,
    158, 110, 200, 246, 74, 226, 30, 223, 142, 138, 133, 194, 30, 254, 132, 236,
    227, 130, 162, 184, 215, 227, 81, 211, 134, 73, 118, 71, 219, 163, 243, 41,
    118, 21, 155, 87, 11, 53, 153, 130, 178, 126, 151, 86, 225, 36, 251, 130,
  ]);

  test.it("Should mint token", async () => {
    await initTestEnvIfNeeded();
    await requestAirdrop(getPayer().publicKey);

    const mintAmount = 100;
    const mintAuthority = getPayer().publicKey.toBase58();
    const mintTo = mintAuthority;
    const mintAddress = await createTestMint();
    return test
      .stdout()
      .command([
        "mint-to",
        `--amount=${mintAmount}`,
        `--mint=${mintAddress}`,
        `--mint-authority=${mintAuthority}`,
        `--to=${mintTo}`,
      ])
      .it(
        `mint-to ${mintAmount} tokens to ${mintTo} from ${mintAddress} with authority ${mintAuthority}`,
        (ctx: any) => {
          expect(ctx.stdout).to.contain("mint-to successful");
        },
      );
  });

  test.it(
    "Should allow authority that is not the payer to mint tokens",
    async () => {
      await initTestEnvIfNeeded();
      await requestAirdrop(getPayer().publicKey);

      const mintAmount = 100;
      const mintAuthority = MINT_AUTHORITY.publicKey.toBase58();
      const mintTo = FIXED_BOB.publicKey.toBase58();
      const mintAddress = await createTestMint();
      return test
        .stdout()
        .command([
          "mint-to",
          `--amount=${mintAmount}`,
          `--mint=${mintAddress}`,
          `--mint-authority=${mintAuthority}`,
          `--to=${mintTo}`,
        ])
        .it(
          `mint-to ${mintAmount} tokens to ${mintTo} from ${mintAddress} with authority ${mintAuthority}`,
          (ctx: any) => {
            expect(ctx.stdout).to.contain("mint-to successful");
          },
        );
    },
  );

  async function createTestMint() {
    const connection = new Connection(getSolanaRpcUrl(), "finalized");
    const { mint, transactionSignature } = await createMint(
      connection,
      getPayer(),
      getPayer().publicKey,
      9,
    );
    await confirmTx(connection, transactionSignature);
    return mint;
  }
});
