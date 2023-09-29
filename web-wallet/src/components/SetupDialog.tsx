//@ts-check
import React, { useState } from "react";
import "../css/index.css";
import { SIGN_MESSAGE } from "../constants";
import {
  Button,
  majorScale,
  Pane,
  Spinner,
  Heading,
  Strong,
  InfoSignIcon,
  Tooltip,
  Paragraph,
  Dialog,
} from "evergreen-ui";
import { useConnection, useWallet } from "@solana/wallet-adapter-react";
import { SignMessageButton } from "./SignMessageButton";
import { createAccount } from "../api/account";
import { PublicKey } from "@solana/web3.js";
import { useAtom } from "jotai";
import {
  fetchedUserAccountAtom,
  isLoggedInAtom,
  isRegisteredAtom,
  isWalletConnectedAtom,
  userKeypairsAtom,
} from "../state/userAtoms";
import { bufferAtom } from "../state/utilAtoms";
import { setErrorAtom } from "../state/errorAtoms";
import { currentSetupStepAtom, SetupStep } from "../state/setupAtoms";
import { publicSolBalanceAtom } from "../state/balancesAtoms";

const Message0 = () => (
  <Paragraph size={400}>
    Welcome to Light!
    <br />
    The Light Privacy Shield has <strong>shielded addresses</strong> at its
    core. <br />
    Your shielded address allows you to wrap (<strong>shield</strong>) your
    tokens into your own private balance. When you unwrap (
    <strong>unshield</strong>) back into regular Solana addresses your on-chain
    privacy is protected.
    <br />
  </Paragraph>
);

const Message1 = () => (
  <>
    <Paragraph>
      Light stores your shielded address on-chain so others can send you tokens
      privately.
      <Tooltip content="Light stores your current wallet address and links it to your wallet's shielded address.">
        <InfoSignIcon color="muted" marginLeft="4px" size={13} />
      </Tooltip>
      <Pane
        marginTop={majorScale(1)}
        display="flex"
        alignItems="center"
        justifyContent="space-between"
      >
        <Pane display="flex" alignItems="baseline">
          <Heading size={400} marginRight="4px">
            <Strong>Registration fees</Strong>
          </Heading>
          <Tooltip content="These are network fees for on-chain storage + transaction fees">
            <InfoSignIcon size={13} />
          </Tooltip>
        </Pane>
        <Heading size={400}>
          <Strong>0.00157 Sol</Strong>
        </Heading>
      </Pane>
    </Paragraph>
  </>
);

const ModalMessage = ({
  publicKey,
  setupStep,
}: {
  publicKey: PublicKey;
  setupStep: number;
}) => {
  const [publicSolBalance] = useAtom(publicSolBalanceAtom);
  return (
    <Pane
      display="flex"
      flexDirection="column"
      marginTop={majorScale(1)}
      marginBottom={majorScale(5)}
    >
      <Pane marginBottom={majorScale(1)}>
        <Heading marginBottom={majorScale(1)} size={200}>
          {publicKey && `Your wallet: ${publicKey.toBase58()}`}
        </Heading>
        <Heading marginBottom={majorScale(1)} size={200}>
          {publicKey &&
            publicSolBalance &&
            `Your public balance: ${publicSolBalance.uiAmount} SOL`}
        </Heading>
      </Pane>
      {setupStep === SetupStep.DERIVE_ACCOUNT && <Message0 />}
      {setupStep === SetupStep.STORE_ACCOUNT && <Message1 />}
    </Pane>
  );
};

export const SetupDialog = () => {
  const { publicKey, sendTransaction } = useWallet();
  const { connection } = useConnection();
  const [setupSuccess, setSetupSuccess] = useState(false);
  const [isLoading, setLoading] = React.useState(false);
  const [currentSetupStep] = useAtom(currentSetupStepAtom);
  const [isLoggedIn] = useAtom(isLoggedInAtom);
  const [isRegistered] = useAtom(isRegisteredAtom);
  const [isWalletConnected] = useAtom(isWalletConnectedAtom);
  const [userKeypairs] = useAtom(userKeypairsAtom);
  const [_, setBuffer] = useAtom(bufferAtom);
  const [__, fetchUserAccount] = useAtom(fetchedUserAccountAtom);
  const [___, setError] = useAtom(setErrorAtom);
  const onClick = () => {
    (async () => {
      if (currentSetupStep === SetupStep.STORE_ACCOUNT) {
        setBuffer(true);
        setLoading(true);
        await createAccount({
          publicKey,
          connection,
          sendTransaction,
          skp: userKeypairs.spendingKeypair,
          ekp: userKeypairs.viewingKeypair,
        }).then(
          (res) => {
            setSetupSuccess(true);
            setLoading(false);
            window.setTimeout(() => {
              setBuffer(false);
              fetchUserAccount(connection);
            }, 2100);
          },
          (error) => {
            setLoading(false);
            setBuffer(false);
            setSetupSuccess(false);
            setError(error?.message);
          },
        );
      }
    })();
  };

  return (
    <>
      <Pane>
        <Dialog
          isShown={true}
          title={
            currentSetupStep === SetupStep.DERIVE_ACCOUNT
              ? "Step 1/2"
              : "Step 2/2"
          }
          shouldCloseOnOverlayClick={false}
          shouldCloseOnEscapePress={false}
          hasCancel={false}
          hasClose={false}
          hasFooter={false}
        >
          <Pane
            display="flex"
            flexDirection="column"
            marginTop={majorScale(2)}
            paddingBottom={majorScale(3)}
          >
            <form
              onSubmit={(e) => {
                e.preventDefault();
                onClick();
              }}
            >
              {isLoading ? (
                <Pane
                  display="flex"
                  alignItems="center"
                  justifyContent="center"
                  height={200}
                >
                  <Spinner />
                </Pane>
              ) : (
                !setupSuccess && (
                  <ModalMessage
                    publicKey={publicKey}
                    setupStep={currentSetupStep}
                  />
                )
              )}
              {setupSuccess && (
                <>
                  <Pane
                    display="flex"
                    alignItems="center"
                    justifyContent="center"
                  >
                    <Strong size={500}>Registration successful!</Strong>
                  </Pane>
                  <Pane
                    height={100}
                    display="flex"
                    alignItems="center"
                    justifyContent="center"
                  >
                    {/*FIXME: must find alternative to Checkmark}
                    {/* <Checkmark size="52px" color="#0066FF" /> */}
                    {/* //"#223344" /> */}
                  </Pane>
                </>
              )}
              {!isRegistered &&
              isWalletConnected &&
              !isLoading &&
              !isLoggedIn &&
              currentSetupStep == SetupStep.DERIVE_ACCOUNT ? (
                <SignMessageButton
                  label="Generate Shielded Address"
                  message={SIGN_MESSAGE}
                />
              ) : (
                !isRegistered &&
                isWalletConnected &&
                !isLoading &&
                isLoggedIn &&
                !setupSuccess &&
                currentSetupStep == SetupStep.STORE_ACCOUNT && (
                  <Button
                    type="submit"
                    appearance="primary"
                    size="large"
                    width="-webkit-fill-available"
                  >
                    Register Shielded Address
                  </Button>
                )
              )}

              {!isRegistered && isWalletConnected && isLoading && (
                <Button
                  disabled
                  appearance="primary"
                  size="large"
                  width="-webkit-fill-available"
                >
                  Waiting for signature ...
                </Button>
              )}
            </form>
          </Pane>
        </Dialog>
      </Pane>
    </>
  );
};
