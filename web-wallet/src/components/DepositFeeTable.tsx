import "../css/index.css";
import {
  majorScale,
  Pane,
  Text,
  Heading,
  Strong,
  minorScale,
  Tooltip,
  InfoSignIcon,
} from "evergreen-ui";

import {
  DEPOSIT_SIG_FEES,
  DEPOSIT_RENT_FEES,
  DECIMALS_SOL,
  Token,
} from "../constants";
import { intToFloat } from "../util/helpers";
import { useAtom } from "jotai";

import { Prices, priceOracleAtom } from "../state/priceOracleAtoms";
import {
  activeFeeConfigAtom,
  activeTokenAtom,
  activeConversionRateAtom,
  activePublicBalanceAtom,
} from "../state/activeAtoms";
import { ActiveFeeConfig } from "../state/userUtxoAtoms";

export const DepositFeeTable = ({
  baseUnitAmount = 0,
  showWarning = false,
  setFormAmount,
}: {
  baseUnitAmount?: number;
  showWarning?: boolean;
  setFormAmount: Function;
}) => {
  const [activeFeeConfig] = useAtom(activeFeeConfigAtom);
  const [activeToken] = useAtom(activeTokenAtom);
  const [conversionRate] = useAtom(activeConversionRateAtom);
  const [priceOracle] = useAtom(priceOracleAtom);

  let ratio = baseUnitAmount / activeFeeConfig.TOTAL_FEES_DEPOSIT;

  return (
    <Pane
      display="flex"
      flexDirection="column"
      textAlign="left"
      marginBottom={majorScale(2)}
      marginTop={majorScale(2)}
    >
      <ShieldAmount
        baseUnitAmount={baseUnitAmount}
        activeFeeConfig={activeFeeConfig}
        activeToken={activeToken}
        conversionRate={conversionRate}
      />
      <Fees priceOracle={priceOracle} activeFeeConfig={activeFeeConfig} />
      <Pane display="flex" flexDirection="column" marginTop={majorScale(3)}>
        <LowRatioWarning
          activeToken={activeToken}
          showWarning={showWarning}
          ratio={ratio}
        />
        <ExceededLimitWarning
          setFormAmount={setFormAmount}
          activeFeeConfig={activeFeeConfig}
          baseUnitAmount={baseUnitAmount}
          activeToken={activeToken}
        />
        <LessThanMinimumWarning
          activeFeeConfig={activeFeeConfig}
          baseUnitAmount={baseUnitAmount}
          activeToken={activeToken}
        />
      </Pane>
    </Pane>
  );
};

const ShieldAmount = ({
  baseUnitAmount,
  activeFeeConfig,
  activeToken,
  conversionRate,
}: {
  baseUnitAmount: number;
  activeFeeConfig: ActiveFeeConfig;
  activeToken: string;
  conversionRate: number;
}) => {
  const { DECIMALS: decimals } = activeFeeConfig;
  const shieldUiAmount = baseUnitAmount / decimals;
  return (
    <>
      <Pane
        marginTop={majorScale(1)}
        display="flex"
        justifyContent="space-between"
      >
        <Pane minWidth="fit-content" display="flex" alignItems="start">
          <Heading size={400} marginRight="4px">
            <Strong>You shield</Strong>
          </Heading>
        </Pane>
        <Pane display="flex" flexDirection="column" alignItems="flex-end">
          <Heading size={400}>
            <Strong>
              {shieldUiAmount} {activeToken}
            </Strong>
          </Heading>
          {conversionRate && (
            <Heading size={200}>
              ($
              {Number(
                intToFloat(conversionRate * (baseUnitAmount / decimals), 2),
              )}
              )
            </Heading>
          )}
        </Pane>
      </Pane>
    </>
  );
};

const Fees = ({
  priceOracle,
  activeFeeConfig,
}: {
  priceOracle: Prices;
  activeFeeConfig: ActiveFeeConfig;
}) => {
  var { TOTAL_FEES_DEPOSIT, DEPOSIT_COLLATERAL } = activeFeeConfig;

  // Always SOL as fees for shield!
  const depositSigFeesUiAmount = DEPOSIT_SIG_FEES / DECIMALS_SOL;
  const depositRentFeesUiAmount = DEPOSIT_RENT_FEES / DECIMALS_SOL;
  const transactionFees = Number(intToFloat(depositSigFeesUiAmount, 4));

  const transactionFeesInUsd = Number(
    intToFloat(priceOracle.usdPerSol * depositSigFeesUiAmount, 2),
  );

  const storageCost = Number(intToFloat(depositRentFeesUiAmount, 4));
  const storageCostInUsd = Number(
    intToFloat(priceOracle.usdPerSol * depositRentFeesUiAmount, 2),
  );

  const totalFees = Number(intToFloat(TOTAL_FEES_DEPOSIT / DECIMALS_SOL, 4));
  const collateralUiAmount = DEPOSIT_COLLATERAL / DECIMALS_SOL;
  return (
    <>
      {" "}
      <Pane
        marginTop={majorScale(2)}
        display="flex"
        justifyContent="space-between"
        alignItems="baseline"
      >
        <Pane minWidth="fit-content" display="flex" alignItems="center">
          <Heading size={400} marginRight="4px">
            <Strong>Total fees</Strong>
          </Heading>
          <Tooltip
            content={`Total fees to pay on top of your shielding baseUnitAmount. Note that if the shielding fails you only get back the fees for transactions that haven't been processed yet. For unshields you will pay far less fees, but they will be deducted from your shielded balance, so make sure you shield enough. A temporary account collateral of ${collateralUiAmount} SOL is required. You get the collateral back after the shielding.`}
          >
            <InfoSignIcon size={13} />
          </Tooltip>
        </Pane>
        <Pane display="flex" flexDirection="column" alignItems="flex-end">
          <Heading size={400}>
            <Strong>{totalFees} SOL</Strong>
          </Heading>
          <Pane textAlign="end">
            <Heading size={200}>
              transaction fees {transactionFees} SOL
              {priceOracle.usdPerSol && (
                <Strong size={300} color="#0066ff">
                  (${transactionFeesInUsd})
                </Strong>
              )}{" "}
              <br />
              storage cost {storageCost} SOL
              {priceOracle.usdPerSol && (
                <Strong size={300} color="#0066ff">
                  {" "}
                  ($
                  {storageCostInUsd})
                </Strong>
              )}{" "}
              <br />
            </Heading>
          </Pane>
        </Pane>
      </Pane>
    </>
  );
};

const LowRatioWarning = ({
  activeToken,
  showWarning,
  ratio,
}: {
  activeToken: string;
  showWarning: boolean;
  ratio: number;
}) => {
  const isLowRatioSol = activeToken === Token.SOL && showWarning && ratio < 2;
  return (
    <>
      {isLowRatioSol && (
        <Text
          marginBottom={minorScale(1)}
          size={300}
          color="red"
          marginRight="4px"
        >
          The baseUnitAmount-to-fee ratio is very low (
          {String(ratio).substring(0, 4)}
          x)
        </Text>
      )}
    </>
  );
};

const ExceededLimitWarning = ({
  activeFeeConfig,
  baseUnitAmount,
  activeToken,
  setFormAmount,
}: {
  activeFeeConfig: ActiveFeeConfig;
  baseUnitAmount: number;
  activeToken: string;
  setFormAmount: Function;
}) => {
  const { MAXIMUM_SHIELD_AMOUNT, DECIMALS: decimals } = activeFeeConfig;
  const [activePublicBalance] = useAtom(activePublicBalanceAtom);

  return (
    <>
      {baseUnitAmount > MAXIMUM_SHIELD_AMOUNT && (
        <Pane display="flex" alignItems="baseline">
          <Text
            marginBottom={minorScale(1)}
            size={300}
            color="blue"
            marginRight="4px"
          >
            Maximum baseUnitAmount per shield is currently{" "}
            {MAXIMUM_SHIELD_AMOUNT / decimals} {activeToken}
          </Text>
          <Text
            cursor="pointer"
            size={300}
            textDecoration="underline"
            color="muted"
            onMouseOver={(e: any) => (e.target.style.color = "blue")}
            onMouseLeave={(e: any) => (e.target.style.color = "#696f8c")}
            marginLeft="2px"
            onClick={(e: any) => {
              e.preventDefault();

              if (activePublicBalance.amount > MAXIMUM_SHIELD_AMOUNT) {
                setFormAmount(MAXIMUM_SHIELD_AMOUNT / decimals);
              }
            }}
          >
            Choose: {MAXIMUM_SHIELD_AMOUNT / decimals} {activeToken}
          </Text>
        </Pane>
      )}
    </>
  );
};

const LessThanMinimumWarning = ({
  activeFeeConfig,
  baseUnitAmount,
  activeToken,
}: {
  activeFeeConfig: ActiveFeeConfig;
  baseUnitAmount: number;
  activeToken: string;
}) => {
  const { MINIMUM_SHIELD_AMOUNT, DECIMALS: decimals } = activeFeeConfig;
  return (
    <>
      {" "}
      {baseUnitAmount > 0 && baseUnitAmount < MINIMUM_SHIELD_AMOUNT && (
        <Text
          marginBottom={minorScale(1)}
          size={300}
          color="blue"
          marginRight="4px"
        >
          Minimum shield baseUnitAmount is {MINIMUM_SHIELD_AMOUNT / decimals}{" "}
          {activeToken}
        </Text>
      )}
    </>
  );
};
