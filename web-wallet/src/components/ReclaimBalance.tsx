import { useEffect, useState } from "react";
import { Pane, Heading, InlineAlert, toaster } from "evergreen-ui";
import { claimFunds } from "../util/claimFunds";
import { DECIMALS_SOL } from "../constants";
import {
  Connection,
  PublicKey,
  Keypair as SolanaKeypair,
} from "@solana/web3.js";
import { useAtom } from "jotai";
import { addPublicBurnerBalanceAtom } from "../state/balancesAtoms";

export const ReclaimBalance = ({
  bkp,
  burnerBalance,
  publicKey,
  connection,
  verificationPda,
}: {
  bkp: SolanaKeypair;
  burnerBalance: number;
  verificationPda: PublicKey;
  publicKey: PublicKey;
  connection: Connection;
}) => {
  const [isResetting, setIsResetting] = useState(false);
  const [_, fetchBurnerBalance] = useAtom(addPublicBurnerBalanceAtom);

  async function reclaim() {
    try {
      setIsResetting(true);
      let rem = await claimFunds(publicKey, bkp, connection, null);
      if (rem || rem === 0) {
        toaster.notify(
          "The remaining fees of a prior shielding has been transferred back into your wallet",
          {
            duration: 8,
          },
        );
      }
      setIsResetting(false);
      fetchBurnerBalance(connection);
    } catch (e) {
      toaster.warning(
        `Something went wrong while transferring your shielding funds into your wallet: ${e}`,
        {
          duration: 15,
        },
      );
      setIsResetting(false);
    }
  }

  return (
    <>
      <Pane>
        <Pane display="flex" flexDirection="column">
          <InlineAlert intent="none" marginBottom={48}>
            <Pane display="flex" alignItems="baseline">
              <Heading size={300}>
                {verificationPda
                  ? `You have an active shielding with ${
                      burnerBalance / DECIMALS_SOL
                    } 
                        SOL pending.`
                  : `You have a stalled shielding with ${
                      burnerBalance / DECIMALS_SOL
                    } 
                        SOL pending.`}
              </Heading>
              {isResetting ? (
                <Heading marginLeft="8px" size={300}>
                  Resetting...
                </Heading>
              ) : (
                <Heading
                  size={300}
                  marginLeft="8px"
                  textDecoration="underline"
                  cursor="pointer"
                  onClick={() => {
                    (async () => {
                      await reclaim();
                    })();
                  }}
                >
                  Reset now
                </Heading>
              )}
            </Pane>
          </InlineAlert>
        </Pane>
      </Pane>
    </>
  );
};
