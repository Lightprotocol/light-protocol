import { useState, useEffect } from "react";
import { useWallet } from "@solana/wallet-adapter-react";
import {
  Heading,
  Link,
  minorScale,
  Text,
  majorScale,
  Pane,
  Strong,
  Tooltip,
  InfoSignIcon,
  ChevronDownIcon,
  ChevronUpIcon,
} from "evergreen-ui";
import { intToFloat } from "../util/helpers";
import { useAtom } from "jotai";
import {
  privacyDetailsAtom,
  recipientAtaIsInitializedAtom,
  switchPrivacyDetailsAtom,
} from "../state/utilAtoms";
import { setErrorAtom } from "../state/errorAtoms";
import { userKeypairsAtom, isLoggedInAtom } from "../state/userAtoms";
import { shieldsAtom, unshieldsAtom } from "../state/transactionsAtoms";
import {
  activeTokenAtom,
  activeFeeConfigAtom,
  activeShieldedBalanceAtom,
  activeConversionRateAtom,
} from "../state/activeAtoms";
import { DECIMALS, FEE_USDC, FEE_USDC_PLUS_ATA, Token } from "../constants";

export const WithdrawalFeeTable = ({
  amount = 0,
  recipient = null,
}: {
  amount: number;
  recipient: string | null;
}) => {
  const [activeToken] = useAtom(activeTokenAtom);
  const [activeFeeConfig] = useAtom(activeFeeConfigAtom);
  const [activeShieldedBalance] = useAtom(activeShieldedBalanceAtom);
  const [userKeypairs] = useAtom(userKeypairsAtom);
  const [shields] = useAtom(shieldsAtom);
  const [unshields] = useAtom(unshieldsAtom);
  const [scoreRef, setScoreRef] = useState(false);
  const [ttp, setTTP] = useState(null);
  const [noShow] = useState(false);
  const [showStopper] = useState(false);
  const [tenPctWarn] = useState(false);
  const { publicKey } = useWallet();
  const [isLoggedIn] = useAtom(isLoggedInAtom);
  const [activeConversionRate] = useAtom(activeConversionRateAtom);
  const [showPrivacyDetails] = useAtom(privacyDetailsAtom);
  const [_, setError] = useAtom(setErrorAtom);
  const [__, switchPrivacyDetails] = useAtom(switchPrivacyDetailsAtom);
  const [recipientAtaIsInitialized] = useAtom(recipientAtaIsInitializedAtom);
  const { DECIMALS: decimals, TOTAL_FEES_WITHDRAWAL: unshieldFees } =
    activeFeeConfig;
  var myWorker = new Worker("../api/workers/privacyScoreWorker.js", {
    name: "two",
    type: "module",
  });
  useEffect(() => {
    (async () => {
      if (isLoggedIn && shields.length > 0) {
        // if (state.tvl * 0.3 < amount) {
        //   setNoShow(true);
        //   setShowStopper(true);
        //   setTenPctWarn(false);
        // } else {
        //   setNoShow(false);
        //   setShowStopper(false);
        // }

        let data = {
          refAmount: amount,
          refPublicKey: userKeypairs.burnerKeypair!.publicKey.toBase58(),
          shields: shields,
          unshields: unshields,
          tvl: false, // TODO: add tvl back
        };

        myWorker.postMessage(data);
      }
    })();
  }, [isLoggedIn, shields, unshields, amount, activeToken]);

  useEffect(() => {
    if (publicKey) {
      myWorker.onerror = (err) => {
        console.error("Error in Worker");
        setError(err.message);
      };
      myWorker.onmessage = (m) => {
        let { privscore, ttp } = m.data;
        setScoreRef(privscore);
        // if (privscore === "bad") {
        //   set;
        // }

        // if (
        //   state.tvl * 0.1 < amount &&
        //   state.tvl * 0.3 > amount &&
        //   (privscore === "good" || privscore === "ok")
        // ) {
        //   setNoShow(true);
        //   setShowStopper(false);
        //   setTenPctWarn(true);
        // }
        setTTP(ttp);
      };
    }
  });

  return (
    <Pane
      display="flex"
      flexDirection="column"
      textAlign="left"
      marginBottom={majorScale(2)}
      marginTop={majorScale(2)}
    >
      {recipient && (
        <Pane
          marginBottom={majorScale(1)}
          display="flex"
          justifyContent="space-between"
          alignItems="baseline"
        >
          <Pane minWidth="fit-content" display="flex" alignItems="center">
            <Heading size={400} marginRight="4px">
              <Strong>To</Strong>
            </Heading>
            <Tooltip
              content={`This is the address you unshield ${activeToken} to. There will be no on-chain link between your shielded assets and the recipient address.`}
            >
              <InfoSignIcon size={13} />
            </Tooltip>
          </Pane>
          <Heading size={200}>
            <Strong size={300}>
              {recipient.slice(0, 8)}..
              {recipient.slice(recipient.length - 9)}
            </Strong>
          </Heading>
        </Pane>
      )}
      <Pane
        marginTop={majorScale(1)}
        display="flex"
        alignItems="start"
        justifyContent="space-between"
      >
        <Pane display="flex" alignItems="baseline">
          <Heading size={400} marginRight="4px">
            <Strong>Recipient gets</Strong>
          </Heading>
        </Pane>
        <Pane display="flex" flexDirection="column" alignItems="flex-end">
          <Heading size={400}>
            <Strong>
              {amount / decimals} {activeToken}
            </Strong>
          </Heading>
          {activeConversionRate && (
            <Heading size={200}>
              ($
              {Number(
                intToFloat(activeConversionRate * (amount / decimals), 2),
              )}
              )
            </Heading>
          )}
        </Pane>
      </Pane>
      <Pane
        marginTop={majorScale(2)}
        display="flex"
        justifyContent="space-between"
        alignItems="baseline"
      >
        <Pane minWidth="fit-content" display="flex" alignItems="center">
          <Heading size={400} marginRight="4px">
            <Strong>Fees</Strong>
          </Heading>
          <Tooltip
            content={`Transaction fees for compute and on-chain storage. Note that the fees will be deducted from your shielded balance, not your Solana wallet.`}
          >
            <InfoSignIcon size={13} />
          </Tooltip>
        </Pane>
        <Pane display="flex" flexDirection="column" alignItems="flex-end">
          <Heading size={400}>
            <Strong>
              {Number(intToFloat(unshieldFees / decimals, 4))} {activeToken}
            </Strong>
          </Heading>
          <Pane textAlign="end">
            <Heading size={200}>
              {activeConversionRate && (
                <Heading size={200}>
                  ($
                  {Number(
                    intToFloat(
                      activeConversionRate * (unshieldFees / decimals),
                      3,
                    ),
                  )}
                  )
                </Heading>
              )}{" "}
            </Heading>
          </Pane>
          {activeToken === Token.USDC && !recipientAtaIsInitialized && (
            <Pane textAlign="end">
              <Heading size={200}>
                {activeConversionRate && (
                  <Heading color="#06f" size={200}>
                    token account rent:{" "}
                    {(FEE_USDC_PLUS_ATA - FEE_USDC) / DECIMALS} {Token.USDC}
                  </Heading>
                )}{" "}
                <br />
              </Heading>
            </Pane>
          )}
        </Pane>
      </Pane>
      {publicKey && (
        <>
          {!noShow &&
            activeShieldedBalance &&
            activeShieldedBalance?.amount > 0 && (
              <>
                <Pane
                  display="flex"
                  alignItems="center"
                  justifyContent="space-between"
                >
                  {ttp !== null && (
                    <Pane
                      minWidth="fit-content"
                      display="flex"
                      alignItems="center"
                    >
                      <Pane
                        cursor="pointer"
                        display="flex"
                        alignItems="center"
                        onClick={() => switchPrivacyDetails()}
                      >
                        {/* @ts-ignore */}
                        {scoreRef === "good" ? (
                          <Text color="green">
                            <b>Good Privacy</b>
                          </Text>
                        ) : // @ts-ignore
                        scoreRef === "ok" ? (
                          <Text color="#0066ff">
                            <b>Moderate Privacy</b>
                          </Text>
                        ) : // @ts-ignore
                        scoreRef === "bad" ? (
                          <Text color="red">
                            <b>Bad Privacy</b>
                          </Text>
                        ) : (
                          ""
                        )}
                        {!showPrivacyDetails && (
                          <ChevronDownIcon
                            marginLeft="4px"
                            color="gray"
                            size={12}
                          />
                        )}
                        {showPrivacyDetails && (
                          <ChevronUpIcon
                            marginLeft="4px"
                            color="gray"
                            size={12}
                          />
                        )}
                      </Pane>
                    </Pane>
                  )}
                </Pane>
                {showPrivacyDetails && ttp !== null && (
                  <>
                    {/* @ts-ignore */}
                    {scoreRef === "good" ? (
                      <Text color="green">
                        <Text
                          marginBottom={minorScale(0)}
                          marginTop={majorScale(1)}
                          size={300}
                          color="green"
                          marginRight="4px"
                        >
                          This unshield amount is likely to offer strong
                          privacy. This is an indication, not a guarantee.
                          <Link
                            size={300}
                            marginLeft="4px"
                            textDecoration="underline"
                            color={"neutral"}
                            target="_blank"
                            href="https://docs.lightprotocol.com/end-user-guides/effective-privacy#privacy-scores-and-recommendations"
                          >
                            Learn more
                          </Link>{" "}
                        </Text>
                      </Text>
                    ) : // @ts-ignore
                    scoreRef === "ok" ? (
                      <Text
                        marginBottom={minorScale(0)}
                        marginTop={majorScale(1)}
                        size={300}
                        color="#0066ff"
                        marginRight="4px"
                      >
                        This unshield amount is likely to offer moderate
                        privacy. This is an indication, not a guarantee. For
                        stronger privacy we recommend to wait up to another{" "}
                        <b>{Number(ttp + 1).toFixed(0)}h</b>, or try a different
                        amount.
                        <Link
                          size={300}
                          marginLeft="4px"
                          textDecoration="underline"
                          color={"neutral"}
                          target="_blank"
                          href="https://docs.lightprotocol.com/end-user-guides/effective-privacy#privacy-scores-and-recommendations"
                        >
                          Learn more
                        </Link>{" "}
                      </Text>
                    ) : // @ts-ignore
                    scoreRef === "bad" ? (
                      <Text
                        marginBottom={minorScale(0)}
                        marginTop={majorScale(1)}
                        size={300}
                        color="red"
                        marginRight="4px"
                      >
                        This amount offers only weak privacy. We recommend to
                        wait at least another{" "}
                        <b>{Number(ttp + 1).toFixed(0)}h</b> for moderate
                        privacy. Or try a different amount.
                        <Link
                          size={300}
                          marginLeft="4px"
                          textDecoration="underline"
                          color={"neutral"}
                          target="_blank"
                          href="https://docs.lightprotocol.com/end-user-guides/effective-privacy#privacy-scores-and-recommendations"
                        >
                          Learn more
                        </Link>{" "}
                      </Text>
                    ) : (
                      ""
                    )}
                  </>
                )}
              </>
            )}
          {showStopper &&
            noShow &&
            activeShieldedBalance &&
            activeShieldedBalance.amount > 0 && (
              <>
                <Pane
                  // marginTop={majorScale(3)}
                  display="flex"
                  alignItems="center"
                  justifyContent="space-between"
                >
                  <Pane
                    cursor="pointer"
                    display="flex"
                    alignItems="center"
                    onClick={() => switchPrivacyDetails()}
                  >
                    <Text color="red">
                      <b>Bad Privacy</b>
                    </Text>
                  </Pane>
                </Pane>
                <Text
                  marginBottom={minorScale(0)}
                  marginTop={majorScale(1)}
                  size={300}
                  color="red"
                  marginRight="4px"
                >
                  You're trying to unshield more than 30% of Light's current
                  TVL. Please keep in mind that this likely offers very low
                  privacy. We recommend to wait until the TVL has increased, or
                  to unshield less at once.
                  <Link
                    size={300}
                    marginLeft="4px"
                    textDecoration="underline"
                    color={"neutral"}
                    target="_blank"
                    href="https://docs.lightprotocol.com/end-user-guides/effective-privacy#privacy-scores-and-recommendations"
                  >
                    Learn more
                  </Link>{" "}
                </Text>
              </>
            )}
          {/* only in case that it would be GOOD or MODERATE! */}
          {tenPctWarn &&
            noShow &&
            activeShieldedBalance &&
            activeShieldedBalance.amount > 0 && (
              <>
                <Pane
                  // marginTop={majorScale(3)}
                  display="flex"
                  alignItems="center"
                  justifyContent="space-between"
                >
                  <Pane
                    cursor="pointer"
                    display="flex"
                    alignItems="center"
                    onClick={() => switchPrivacyDetails()}
                  >
                    <Text color="#0066ff">
                      <b>Moderate Privacy</b>
                    </Text>
                  </Pane>
                </Pane>
                <Text
                  marginBottom={minorScale(0)}
                  marginTop={majorScale(1)}
                  size={300}
                  color="#0066ff"
                  marginRight="4px"
                >
                  You're trying to unshield more than 10% of Light's current
                  TVL. Please keep in mind that this likely offers low privacy.
                  For stronger privacy, we recommend to wait until the TVL has
                  increased, or to unshield less at once.
                  <Link
                    size={300}
                    marginLeft="4px"
                    textDecoration="underline"
                    color={"neutral"}
                    target="_blank"
                    href="https://docs.lightprotocol.com/end-user-guides/effective-privacy#privacy-scores-and-recommendations"
                  >
                    Learn more
                  </Link>{" "}
                </Text>
              </>
            )}
        </>
      )}
    </Pane>
  );
};
