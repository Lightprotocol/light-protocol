import { useState, useEffect } from "react";
import {
  Text,
  majorScale,
  Pane,
  Tooltip,
  InfoSignIcon,
  Position,
} from "evergreen-ui";
import SetAmountButton from "./SetAmountButton";
import { PublicKey } from "@solana/web3.js";
import { shieldsAtom, unshieldsAtom } from "../state/transactionsAtoms";
import { useAtom } from "jotai";
import {
  activeFeeConfigAtom,
  activeTokenAtom,
  activeShieldedBalanceAtom,
} from "../state/activeAtoms";

export const RecommendedAmounts = ({
  setAmount,
  publicKey,
}: {
  setAmount: Function;
  publicKey: PublicKey;
}) => {
  const [shields] = useAtom(shieldsAtom);
  const [unshields] = useAtom(unshieldsAtom);
  const [activeFeeConfig] = useAtom(activeFeeConfigAtom);
  const [activeToken] = useAtom(activeTokenAtom);
  const [activeShieldedBalance] = useAtom(activeShieldedBalanceAtom);
  const [options, setOptions] = useState([]);

  // TODO: move worker thread into own function and return promise
  var myWorker = new Worker("../api/workers/privateAmountSuggestionWorker.js", {
    name: "three",
    type: "module",
  });
  useEffect(() => {
    if (shields.length > 0 && unshields.length > 0) {
      let state = {
        shields: shields,
        unshields: unshields,
        tvl: null,
        userBalance: activeShieldedBalance?.amount,
      };
      let data = {
        state: state,
        publicKey: publicKey.toBase58(),
      };
      myWorker.postMessage(data);
    }
  }, [shields]);

  useEffect(() => {
    if (publicKey) {
      myWorker.onerror = (err) => {
        console.log(`Error in Worker ${err}`);
      };
      myWorker.onmessage = (m) => {
        let { privateAmounts } = m.data;
        setOptions(privateAmounts);
      };
    }
  });
  const { DECIMALS } = activeFeeConfig;
  return (
    <>
      {options.length > 0 && (
        <Pane
          marginTop={majorScale(2)}
          marginBottom={majorScale(4)}
          display="flex"
          alignItems="baseline"
          flexDirection="row"
        >
          <Text marginRight="4px" size={300} color="muted">
            Recommended{" "}
          </Text>
          <Pane marginRight="16px">
            <Tooltip
              position={Position.RIGHT}
              content="Recommended amounts ensure privacy more quickly. The recommendation is randomized and based on live traffic."
            >
              <InfoSignIcon color="muted" size={10} />
            </Tooltip>
          </Pane>
          {options.map((amount) => {
            return (
              <Pane marginRight="12px">
                <SetAmountButton
                  amount={Math.round((amount / DECIMALS) * 10) / 10}
                  setAmount={setAmount}
                  descriptor={activeToken}
                />
              </Pane>
            );
          })}
        </Pane>
      )}
    </>
  );
};
