import Skeleton from "react-loading-skeleton";
import "react-loading-skeleton/dist/skeleton.css";
import "../css/App.css";
import { Pane, majorScale, Heading, Avatar } from "evergreen-ui";
import { useAtom } from "jotai";

import { priceOracleAtom } from "../state/priceOracleAtoms";
import { Token } from "../constants";
import {
  totalUserShieldedBalanceAtom,
  shieldedSolBalanceAtom,
  shieldedUsdcBalanceAtom,
  shieldedBalanceIsFetchedAtom,
} from "../state/balancesAtoms";
import { isUtxoSyncCompleteAtom } from "../state/userUtxoAtoms";
import { round, roundTo } from "../util/helpers";

export const RenderShieldedBalance = () => {
  const [totalBalance] = useAtom(totalUserShieldedBalanceAtom);
  const [shieldedSolBalance] = useAtom(shieldedSolBalanceAtom);
  const [shieldedUsdcBalance] = useAtom(shieldedUsdcBalanceAtom);
  const [shieldedBalanceIsFetched] = useAtom(shieldedBalanceIsFetchedAtom);
  const [isUtxoSyncComplete] = useAtom(isUtxoSyncCompleteAtom);
  const [priceOracle] = useAtom(priceOracleAtom);
  return (
    <Pane display="flex" flexDirection="column" width={majorScale(50)}>
      <Pane display="flex" marginLeft={majorScale(3)} marginBottom="4px">
        <Heading size={100} color="#474d66">
          My shielded balance
        </Heading>
      </Pane>
      <Pane
        background="white"
        borderRadius={majorScale(2)}
        paddingLeft={majorScale(3)}
        paddingRight={majorScale(2)}
        paddingTop={majorScale(2)}
        paddingBottom={majorScale(3)}
        borderBottomLeftRadius={majorScale(0)}
        borderBottomRightRadius={majorScale(0)}
        elevation={1}
        display="flex"
        flexDirection="column"
        justifyContent="start"
        alignItems="start"
      >
        <Pane display="flex" flexDirection="row">
          {isUtxoSyncComplete && priceOracle.usdPerSol ? (
            <Heading size={900} color="#474d66" display="flex">
              ${""}
              {totalBalance === 0 ? totalBalance : round(totalBalance)}
            </Heading>
          ) : (
            <Heading
              width="72px"
              borderRadius="16px"
              size={900}
              color="#474d66"
            >
              <Skeleton count={1} />
            </Heading>
          )}
        </Pane>

        <Pane
          display="flex"
          marginTop="12px"
          marginBottom="4px"
          flexDirection="row"
          alignItems="center"
        >
          {isUtxoSyncComplete ? (
            <Heading
              size={500}
              color="#474d66
  
              "
            >
              {round(shieldedSolBalance!.uiAmount, 6)}
            </Heading>
          ) : (
            <Heading
              width="72px"
              borderRadius="16px"
              size={900}
              color="#474d66"
            >
              <Skeleton count={1} />
            </Heading>
          )}
          <Pane marginLeft="4px" display="flex" alignItems="center">
            <Avatar
              src="https://raw.githubusercontent.com/solana-labs/token-list/main/assets/mainnet/So11111111111111111111111111111111111111112/logo.png"
              name="Solana Icon"
              size={16}
              marginRight="4px"
            />
            <Heading color="#474d66" size={100}>
              {Token.SOL}
            </Heading>
          </Pane>
        </Pane>
        <Pane display="flex" flexDirection="row" alignItems="center">
          {isUtxoSyncComplete && shieldedUsdcBalance ? (
            <Heading
              size={500}
              color="#474d66
  
              "
            >
              {round(shieldedUsdcBalance!.uiAmount, 6)}
            </Heading>
          ) : (
            <Heading
              width="72px"
              borderRadius="16px"
              size={900}
              color="#474d66"
            >
              <Skeleton count={1} />
            </Heading>
          )}
          <Pane marginLeft="4px" display="flex" alignItems="center">
            <Avatar
              src="https://raw.githubusercontent.com/solana-labs/token-list/main/assets/mainnet/EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v/logo.png"
              name="USDC Icon"
              size={16}
              marginRight="4px"
            />
            <Heading color="#474d66" size={100}>
              {Token.USDC}
            </Heading>
          </Pane>
        </Pane>
      </Pane>
      <Pane
        background="white"
        width={majorScale(50)}
        elevation={1}
        borderBottomRightRadius={majorScale(2)}
        borderBottomLeftRadius={majorScale(2)}
        paddingTop={majorScale(3)}
        paddingBottom={majorScale(2)}
      ></Pane>
    </Pane>
  );
};
