//@ts-check
import "../css/index.css";
import { useState, useEffect } from "react";
import { Button, majorScale, Pane, Text, Heading, toaster } from "evergreen-ui";
import { AmountInputForm } from "./AmountInputForm";
import { useConnection, useWallet } from "@solana/wallet-adapter-react";

import { Wallet } from "./Wallet";
import {
  TOTAL_FEES_DEPOSIT,
  DEPOSIT_COLLATERAL,
  DECIMALS_SOL,
  Token,
  USDC_BURNER_ACCOUNT_RENT,
} from "../constants";
import {
  checkAmount,
  checkShieldAmount,
  runFormChecks,
} from "../util/amountValidation";
import { DepositFeeTable } from "./DepositFeeTable";
import { prepareAndSendShield } from "../util/shielding";
import { useAtom } from "jotai";
import {
  userUtxosAtom,
  utxoSyncAtom,
  devalidatedUserUtxosAtom,
  fetchedUserUtxosAtom,
} from "../state/userUtxoAtoms";
import { isWalletConnectedAtom, userKeypairsAtom } from "../state/userAtoms";
import { formInputAmountAtom, bufferAtom } from "../state/utilAtoms";
import { setErrorAtom } from "../state/errorAtoms";
import {
  setProcessDoneAtom,
  setProcessingErrorAtom,
  startPrepareAtom,
  startProcessAtom,
  startSignAtom,
} from "../state/navigationAtoms";
import { verificationPdaAtom } from "../state/utilAtoms";
import {
  activeTokenAtom,
  activeFeeConfigAtom,
  activePublicBalanceAtom,
  activeUnspentUtxosAtom,
} from "../state/activeAtoms";
import {
  fetchedPublicBalancesAtom,
  publicSolBalanceAtom,
  shieldBalanceDisplayAtom,
} from "../state/balancesAtoms";
import { claimFunds } from "../util/claimFunds";
import {
  fetchLeavesAtom,
  fetchNullifiersAtom,
  fetchedTransactionsAtom,
  leavesAtom,
  nullifierAtom,
} from "../state/transactionsAtoms";

export const DepositForm = () => {
  const { publicKey, signAllTransactions } = useWallet();
  const { connection } = useConnection();
  const [formAmount, setFormAmount] = useState<string | number>("");
  const [amountSyntaxError, setAmountSyntaxError] = useState(true);
  const [amountValidityError, setAmountValidityError] = useState(false);
  const [amountThresholdWarning, setAmountThresholdWarning] = useState(false);
  const [amountMinWarning, setAmountMinWarning] = useState(false);
  const [activeToken] = useAtom(activeTokenAtom);
  const [activeFeeConfig] = useAtom(activeFeeConfigAtom);
  const { DECIMALS } = activeFeeConfig;
  const [isWalletConnected] = useAtom(isWalletConnectedAtom);
  const [shieldBalanceDisplay] = useAtom(shieldBalanceDisplayAtom);
  const [activePublicBalance] = useAtom(activePublicBalanceAtom);
  const [publicSolBalance] = useAtom(publicSolBalanceAtom);
  const [userUtxos] = useAtom(userUtxosAtom);
  const [isInitialUtxoSync] = useAtom(utxoSyncAtom);
  const [activeUnspentUtxos] = useAtom(activeUnspentUtxosAtom);
  const [_, setAmountIsOK] = useState(true);
  const [__, setFormInputAmountAtom] = useAtom(formInputAmountAtom);
  const [___, setError] = useAtom(setErrorAtom);
  const [____, startSigning] = useAtom(startSignAtom);
  const [_____, startPreparing] = useAtom(startPrepareAtom);
  const [______, startProcessing] = useAtom(startProcessAtom);
  const [_______, setDone] = useAtom(setProcessDoneAtom);
  const [________, setProcessingError] = useAtom(setProcessingErrorAtom);
  const [_________, setBuffer] = useAtom(bufferAtom);
  const [__________, setVerificationPda] = useAtom(verificationPdaAtom);
  const [___________, devalidateInUtxos] = useAtom(devalidatedUserUtxosAtom);
  const [____________, refetchUserUtxos] = useAtom(fetchedUserUtxosAtom);
  const [_____________, refetchLatestPublicBalances] = useAtom(
    fetchedPublicBalancesAtom,
  );
  const [______________, refetchTransactions] = useAtom(
    fetchedTransactionsAtom,
  );
  const [_______________, fetchLeaves] = useAtom(fetchLeavesAtom);
  const [________________, fetchNullifiers] = useAtom(fetchNullifiersAtom);

  const [userKeypairs] = useAtom(userKeypairsAtom);
  const [{ leavesSol, leavesUsdc }] = useAtom(leavesAtom);
  let baseUnitAmount = Number(formAmount) * DECIMALS;
  let userTokenAccount = activePublicBalance.publicKey;
  useEffect(() => {
    // setFormInputAmountAtom(Number(formAmount) * DECIMALS);

    runFormChecks({
      amount: baseUnitAmount,
      token: activeToken,
      signerBalance: activePublicBalance.amount,
      setAmountSyntaxError,
      setAmountValidityError,
      setAmountThresholdWarning,
      setAmountMinWarning,
      publicSolBalance,
    });
  }, [formAmount, activeToken]);

  const onClick = () => {
    refetchLatestPublicBalances(connection);
    setFormInputAmountAtom(formAmount); // TODO: can make this cleaner
    startSigning();

    (async () => {
      await refetchTransactions();
      await prepareAndSendShield({
        connection,
        publicKey: publicKey!,
        amount: baseUnitAmount,
        signAllTransactions: signAllTransactions!,
        userTokenAccount: userTokenAccount!,
        activeToken,
        process: {
          startSigning,
          startPreparing,
          startProcessing,
          setDone,
          setProcessingError,
        },
        setError,
        setBuffer,
        activeUnspentUtxos,
        userKeypairs,
        setVerificationPda,
        refetchUserUtxos,
        devalidateInUtxos,
        userUtxos,
        refetchTransactions,
        fetchNullifiers,
        fetchLeaves,
        leaves: activeToken === Token.SOL ? leavesSol : leavesUsdc,
        toaster,
      });

      try {
        await claimFunds(
          publicKey,
          userKeypairs.burnerKeypair,
          connection,
          null,
        );
        refetchLatestPublicBalances(connection);
      } catch (e) {
        console.log(e);
      }
    })();
  };

  return (
    <>
      <Pane display="flex" flexDirection="column" marginTop={majorScale(2)}>
        <form
          onSubmit={(e) => {
            e.preventDefault();
            console.log("@FORM SUBMIT", baseUnitAmount);
            let okAmount = checkShieldAmount({
              amount: baseUnitAmount,
              signerBalance: publicSolBalance.amount,
              setError,
              token: activeToken,
            });
            if (String(formAmount).indexOf(",") > -1) {
              okAmount = false;
            }
            if (okAmount) {
              setAmountIsOK(true);
            } else {
              setAmountIsOK(false);
            }
            if (okAmount) {
              onClick();
            }
          }}
        >
          {publicKey && (
            <Pane
              justifyContent="flex-end"
              display="flex"
              alignItems="baseline"
            >
              <Text
                cursor="pointer"
                size={300}
                onClick={() => setFormAmount(shieldBalanceDisplay)}
              >
                Max:
              </Text>
              <Text
                cursor="pointer"
                size={300}
                textDecoration="underline"
                color="muted"
                onMouseOver={(e: any) => (e.target.style.color = "blue")}
                onMouseLeave={(e: any) => (e.target.style.color = "#696f8c")}
                marginLeft="2px"
                onClick={() => {
                  if (shieldBalanceDisplay > 0) {
                    setFormAmount(shieldBalanceDisplay);
                  }
                }}
              >
                {shieldBalanceDisplay > 0
                  ? shieldBalanceDisplay
                  : `Not enough ${activeToken} in wallet`}
              </Text>
            </Pane>
          )}
          <AmountInputForm
            // @ts-ignore
            amount={formAmount}
            setAmount={setFormAmount}
            checkAmount={checkAmount}
            autoFocus
          />

          {formAmount && !amountSyntaxError && (
            <Pane marginTop={majorScale(2)}>
              <DepositFeeTable
                showWarning
                baseUnitAmount={baseUnitAmount}
                setFormAmount={setFormAmount}
              />
            </Pane>
          )}

          {isWalletConnected && (
            <>
              {amountValidityError ||
              amountSyntaxError ||
              amountThresholdWarning ||
              amountMinWarning ||
              (userUtxos.length === 0 && isInitialUtxoSync) ? (
                <Button
                  marginTop={majorScale(4)}
                  disabled
                  type="submit"
                  size="large"
                  width="-webkit-fill-available"
                >
                  {userUtxos.length === 0 && isInitialUtxoSync
                    ? "Syncing balance..."
                    : amountValidityError
                    ? "Not enough SOL"
                    : activePublicBalance.amount === 0
                    ? `No ${activeToken} in wallet`
                    : amountThresholdWarning
                    ? "Reached max limit"
                    : amountMinWarning
                    ? "Please shield more"
                    : "Shield"}
                </Button>
              ) : (
                <Button
                  marginTop={majorScale(4)}
                  type="submit"
                  appearance={"primary"}
                  size="large"
                  width="-webkit-fill-available"
                >
                  Shield now
                </Button>
              )}
            </>
          )}
          {isWalletConnected && amountValidityError && (
            <>
              <Heading marginTop={majorScale(2)} color="#8f95b2" size={500}>
                {`You need at least 
                ${String(
                  (activeToken === Token.SOL
                    ? baseUnitAmount + TOTAL_FEES_DEPOSIT + DEPOSIT_COLLATERAL
                    : TOTAL_FEES_DEPOSIT +
                      DEPOSIT_COLLATERAL +
                      USDC_BURNER_ACCOUNT_RENT) / DECIMALS_SOL,
                ).substring(0, 5)} SOL in your wallet, but you have ${String(
                  publicSolBalance.uiAmount,
                ).substring(0, 5)}.`}
              </Heading>
            </>
          )}
        </form>

        <>
          {!isWalletConnected && (
            <Pane marginTop={majorScale(4)} width={"100%"}>
              <Wallet />
            </Pane>
          )}
        </>
      </Pane>
    </>
  );
};
