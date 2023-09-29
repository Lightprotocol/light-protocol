import { useState, useEffect } from "react";
import { useConnection, useWallet } from "@solana/wallet-adapter-react";
import { prepareTransaction } from "../sdk/src/prepareTransaction";
import { sign } from "tweetnacl";
import {
  Heading,
  Button,
  Text,
  majorScale,
  Pane,
  InlineAlert,
  TextInput,
  toaster,
} from "evergreen-ui";
import { AmountInputForm } from "./AmountInputForm";
import { Wallet } from "./Wallet";
import { Utxo } from "../sdk/src/utxo";
import {
  FEE,
  RELAYER_ADDRESS,
  RELAYER_URL,
  DECIMALS_SOL,
  TransactionType,
  Token,
  FEE_USDC,
  ADMIN_KEYPAIR,
  FEE_USDC_PLUS_ATA,
} from "../constants.js";
import { RecommendedAmounts } from "./RecommendedAmounts";
import { WithdrawalFeeTable } from "./WithdrawalFeeTable";

import {
  matchAdd,
  checkAmount,
  checkAmountLimit,
} from "../util/amountValidation";
import { pickUtxos } from "../util/utxoSelection";
import axios from "axios";
import { uint8ArrayToArray } from "../util/helpers";
import { useAtom } from "jotai";
import {
  formInputAmountAtom,
  formInputRecipientAtom,
  recipientAtaIsInitializedAtom,
  recipientFieldIsValidAtom,
  recipientStateAtom,
} from "../state/utilAtoms";

import {
  setProcessDoneAtom,
  setProcessingErrorAtom,
  startPrepareAtom,
  startProcessAtom,
  startSignAtom,
} from "../state/navigationAtoms";
import { isWalletConnectedAtom, userKeypairsAtom } from "../state/userAtoms";
import { setErrorAtom } from "../state/errorAtoms";
import {
  activeTokenAtom,
  activeFeeConfigAtom,
  activeShieldedBalanceAtom,
  activeUnspentUtxosAtom,
  activeUserUtxosAtom,
} from "../state/activeAtoms";
import {
  devalidatedUserUtxosAtom,
  fetchedUserUtxosAtom,
  userUtxosAtom,
  utxoSyncAtom,
} from "../state/userUtxoAtoms";
import { PublicKey } from "@solana/web3.js";
import { getRelayFee } from "../util/getRelayFee";
import { getAssociatedTokenAccountAndInfo } from "../util/isAssociatedTokenAccount";
import { storeIndicesInLocalStorage } from "../util/persistIndices";
import {
  fetchLeavesAtom,
  fetchNullifiersAtom,
  fetchedTransactionsAtom,
  leavesAtom,
} from "../state/transactionsAtoms";
const { U64 } = require("n64");
const { BigNumber } = require("ethers");

async function signUnshieldMessage({
  baseUnitAmount,
  DECIMALS,
  TOTAL_FEES_WITHDRAWAL,
  activeToken,
  signMessage,
  publicKey,
  setError,
}: {
  baseUnitAmount: number;
  DECIMALS: number;
  TOTAL_FEES_WITHDRAWAL: number;
  activeToken: string;
  signMessage: Function;
  publicKey: PublicKey;
  setError: Function;
}) {
  // Sign message
  let messageAmount = baseUnitAmount / DECIMALS;
  let messageFee = TOTAL_FEES_WITHDRAWAL / DECIMALS;
  const message = new TextEncoder().encode(
    `Please confirm your unshielding of \n${messageAmount} (amount) + ${messageFee} (fees) ${activeToken}`,
  );

  if (!signMessage || !publicKey) {
    toaster.notify("Wallet not connected, please reload your page.");
    return;
  }
  const signature = await signMessage(message);
  if (!sign.detached.verify(message, signature, publicKey.toBytes())) {
    throw new Error("Invalid signature!");
  }
}

export const WithdrawalForm = () => {
  const { connection } = useConnection();
  const { publicKey, signMessage } = useWallet();
  const [activeToken] = useAtom(activeTokenAtom);
  const [activeFeeConfig] = useAtom(activeFeeConfigAtom);
  const [recipientAtom, setRecipientAtom] = useAtom(formInputRecipientAtom);
  const [recipient, setRecipient] = useState(recipientAtom || "");
  const [formAmount, setFormAmount] = useState("");
  const [addFormatValid, setAddFormatValid] = useState(true);
  const [amountLimitError, setAmountLimitError] = useState(false);
  const [amountIsOK, setAmountIsOK] = useState(true);
  const [activeShieldedBalance] = useAtom(activeShieldedBalanceAtom);
  const [userKeypairs] = useAtom(userKeypairsAtom);
  const [activeUnspentUtxos] = useAtom(activeUnspentUtxosAtom);
  const [isWalletConnected] = useAtom(isWalletConnectedAtom);
  const [_, setAmountAtom] = useAtom(formInputAmountAtom);
  const [__, setError] = useAtom(setErrorAtom);
  const [___, startSigning] = useAtom(startSignAtom);
  const [_____, startPreparing] = useAtom(startPrepareAtom);
  const [______, startProcessing] = useAtom(startProcessAtom);
  const [_______, setDone] = useAtom(setProcessDoneAtom);
  const [________, setProcessingError] = useAtom(setProcessingErrorAtom);
  const [___________, devalidateInUtxos] = useAtom(devalidatedUserUtxosAtom);
  const [____________, setRecipientAtaIsInitialized] = useAtom(
    recipientAtaIsInitializedAtom,
  );
  const [activeUserUtxos] = useAtom(activeUserUtxosAtom);
  const [userUtxos] = useAtom(userUtxosAtom);
  const [isInitialUtxoSync] = useAtom(utxoSyncAtom);
  const { TOTAL_FEES_WITHDRAWAL, DECIMALS } = activeFeeConfig;
  const [_____________, refetchUserUtxos] = useAtom(fetchedUserUtxosAtom);
  const [______________, refetchTransactions] = useAtom(
    fetchedTransactionsAtom,
  );
  const [_______________, fetchLeaves] = useAtom(fetchLeavesAtom);
  const [________________, fetchNullifiers] = useAtom(fetchNullifiersAtom);
  const [{ leavesSol, leavesUsdc }] = useAtom(leavesAtom);

  let baseUnitAmount: number = Number(formAmount) * DECIMALS;

  // run checks
  useEffect(() => {
    setAmountLimitError(false);
    setAddFormatValid(true);

    let limitErr = checkAmountLimit(
      activeShieldedBalance?.amount,
      baseUnitAmount,
      activeToken,
    );

    if (limitErr && baseUnitAmount > 0) {
      setAmountLimitError(true);
    }
    if (!limitErr) {
      let ok = matchAdd(recipient);

      if (!ok) {
        setAddFormatValid(false);
      }
    }
  }, [formAmount, recipient, activeToken]);

  const onClick = () => {
    let notesToSend: Utxo[] = pickUtxos(activeUnspentUtxos, baseUnitAmount);
    if (notesToSend.length === 0) {
      setAmountIsOK(false);
      setError("Please try a smaller unshield amount once.");
      return;
    } else {
      setRecipientAtom(recipient);
      setAmountAtom(baseUnitAmount);
    }
    startSigning();

    (async () => {
      await refetchTransactions();
      let inUtxoAmount = notesToSend[0].amount;

      if (notesToSend.length > 1) {
        inUtxoAmount = inUtxoAmount.add(notesToSend[1].amount);
      }
      let ataIsInitialized: boolean = false;
      let associatedTokenAccount: PublicKey = null;
      if (activeToken === Token.USDC) {
        let info = await getAssociatedTokenAccountAndInfo({
          connection,
          recipientField: recipient,
        });
        ataIsInitialized = info.isInitialized;
        associatedTokenAccount = info.associatedTokenAccount;
        if (!ataIsInitialized) {
          setRecipientAtaIsInitialized(false);
        } else {
          setRecipientAtaIsInitialized(true);
        }
      }
      let relayFee = getRelayFee(activeToken, ataIsInitialized);

      let changeUtxo = new Utxo({
        amount: inUtxoAmount
          .sub(BigNumber.from(baseUnitAmount.toString()))
          .sub(BigNumber.from(relayFee.toString())),
        keypair: userKeypairs.spendingKeypair,
      });
      try {
        await signUnshieldMessage({
          baseUnitAmount,
          DECIMALS,
          TOTAL_FEES_WITHDRAWAL,
          activeToken,
          signMessage,
          publicKey,
          setError,
        });
      } catch (e) {
        setError("User rejected the signature request");
        return;
      }

      startPreparing();

      if (activeToken === Token.USDC && !associatedTokenAccount) {
        setError(
          "Whoops. Something went wrong checking the recipient's token account",
        );
        return;
      }
      let {
        data: { proofBytes, publicInputsBytes, extDataBytes },
      } = await prepareTransaction({
        inputs: notesToSend,
        outputs: [changeUtxo],
        fee: U64(relayFee),
        recipient:
          activeToken === Token.USDC
            ? associatedTokenAccount.toBase58()
            : recipient,
        relayer: RELAYER_ADDRESS,
        action: TransactionType.UNSHIELD,
        recipientEncryptionPubkey: userKeypairs.viewingKeypair!.publicKey,
        token: activeToken,
        leaves: activeToken === Token.SOL ? leavesSol : leavesUsdc,
      });

      startProcessing();

      let payload = {
        input: publicInputsBytes,
        proof: proofBytes,
        extData: uint8ArrayToArray(extDataBytes),
        action: TransactionType.UNSHIELD,
        // amount: baseUnitAmount,
        owner: recipient, // always sol address
      };
      try {
        await axios.post(`${RELAYER_URL}/relayunshield`, payload);

        setDone();
        devalidateInUtxos({
          inUtxos: notesToSend,
          outUtxo: { utxo: changeUtxo, token: activeToken, spent: false },
        });
        // storeIndicesInLocalStorage(
        //   userKeypairs.localStorageEncryptionKeypair,
        //   publicKey,
        //   userUtxos,
        // );
        await Promise.all([fetchLeaves(), fetchNullifiers()]);
        await refetchTransactions();
        await refetchUserUtxos();
        toaster.success(`Unshield completed (${formAmount} ${activeToken})`);
      } catch (error: any) {
        console.log("error", error);
        if (
          error === "500" ||
          error.message === "500" ||
          error === "555" ||
          error.message === "555"
        ) {
          /// We must soft fail here.
          setProcessingError();
        } else if (
          (error.response &&
            (error.response || error.response.status) === 524) ||
          error === "Error: Network Error" ||
          error === "Network Error" ||
          error.message === "Error: Network Error" ||
          error.message === "Network Error"
        ) {
          /// Do not fail here. Instead force a refetch until pending == 0
          /// Also (hacky) set unique state.
          // FIXME: replace this somehow. force refetches or so
          setProcessingError();
        } else {
          setProcessingError();
        }
      }
    })();
  };

  return (
    <Pane display="flex" flexDirection="column" marginTop={majorScale(2)}>
      <form
        autoComplete="off"
        onSubmit={(e) => {
          e.preventDefault();
          let matchingAdd = matchAdd(recipient);
          let okAmount = checkAmount({
            userBalance: activeShieldedBalance?.amount,
            withdrawalAmount: baseUnitAmount,
            token: activeToken,
          });
          if (String(baseUnitAmount).indexOf(",") > -1) {
            okAmount = false;
          }
          if (matchingAdd) {
            setAddFormatValid(true);
          } else {
            setAddFormatValid(false);
          }
          if (okAmount) {
            setAmountIsOK(true);
          } else {
            setAmountIsOK(false);
          }
          if (matchingAdd && okAmount) {
            //OkAmount
            onClick();
          }
        }}
      >
        <TextInput
          //@ts-ignore
          label={
            <Text size={300} color="white">
              Recipient
            </Text>
          }
          data-private
          textAlign="left"
          placeholder="Recipient's SOL address"
          value={recipient}
          backgroundColor="#f9fafc"
          width="100%"
          height={"56px"}
          marginBottom={majorScale(1)}
          borderRadius="15px"
          autoFocus
          onChange={(e) => {
            e.preventDefault();
            setRecipient(e.target.value);
            setRecipientAtom(recipient);
          }}
        ></TextInput>

        <Pane marginTop={majorScale(2)}>
          {/*@ts-ignore */}
          <AmountInputForm amount={formAmount} setAmount={setFormAmount} />
        </Pane>
        {!amountIsOK && (
          <InlineAlert intent="danger">
            Please enter a valid amount.
          </InlineAlert>
        )}
        {publicKey && userKeypairs && (
          <RecommendedAmounts
            setAmount={setFormAmount}
            publicKey={userKeypairs.burnerKeypair?.publicKey}
          />
        )}
        {formAmount && Number(formAmount) > 0 && (
          <WithdrawalFeeTable amount={baseUnitAmount} recipient={recipient} />
        )}
        {isWalletConnected && (
          <>
            {amountLimitError ||
            baseUnitAmount == 0 ||
            !addFormatValid ||
            activeUserUtxos.length === 0 ? (
              <Button
                marginTop={majorScale(4)}
                disabled
                type="submit"
                size="large"
                width="-webkit-fill-available"
              >
                {userUtxos.length === 0 && isInitialUtxoSync
                  ? "Syncing balance..."
                  : activeUnspentUtxos.length === 0 && !isInitialUtxoSync
                  ? `No shielded ${activeToken}`
                  : amountLimitError && addFormatValid
                  ? `Not enough shielded ${activeToken}`
                  : "Unshield"}
              </Button>
            ) : (
              <Button
                marginTop={majorScale(4)}
                type="submit"
                appearance={"primary"}
                size="large"
                width="-webkit-fill-available"
              >
                Unshield
              </Button>
            )}
          </>
        )}
        {!isWalletConnected && (
          <Pane marginTop={majorScale(4)}>
            <Wallet />
          </Pane>
        )}
        {amountLimitError && (
          <>
            <Heading marginTop={majorScale(2)} color="#8f95b2" size={500}>
              {`A shielded balance of at least 
                ${String(
                  activeToken === Token.SOL
                    ? (baseUnitAmount + TOTAL_FEES_WITHDRAWAL) / DECIMALS_SOL
                    : (baseUnitAmount + TOTAL_FEES_WITHDRAWAL) / DECIMALS,
                ).substring(0, 7)} ${activeToken} is required.`}
            </Heading>
          </>
        )}
        {!amountLimitError && baseUnitAmount > 0 && !addFormatValid && (
          <>
            <Heading marginTop={majorScale(2)} color="#8f95b2" size={500}>
              {`Please enter a valid recipient SOL address.`}
            </Heading>
          </>
        )}
      </form>
    </Pane>
  );
};
