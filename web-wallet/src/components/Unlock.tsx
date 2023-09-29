import "../css/index.css";
import React from "react";
import { SignMessageButton } from "./SignMessageButton";
import { Button, majorScale, Pane, Heading, Spinner } from "evergreen-ui";
import { Wallet } from "./Wallet";
import logo from "../assets/svg/LogoSecondary.svg";
import { SIGN_MESSAGE } from "../constants";
import { useAtom } from "jotai";
import { isWalletConnectedAtom } from "../state/userAtoms";

export const Unlock = () => {
  const [isWalletConnected] = useAtom(isWalletConnectedAtom);
  const [isLoading, setLoading] = React.useState(false);
  const onClick = () => {
    setLoading(true);
  };

  return (
    <>
      <Pane display="flex" flexDirection="column" marginTop={majorScale(5)}>
        <Pane marginBottom="32px" width="-webkit-fill-available">
          {/* @ts-ignore */}
          <img height="55px" length="40" src={logo} />
        </Pane>
        {!isLoading && (
          <Pane>
            <Heading color="#505050" size={800}>
              Welcome back!
            </Heading>
          </Pane>
        )}
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
              height={90}
            >
              <Spinner />
            </Pane>
          ) : (
            <Pane
              display="flex"
              flexDirection="row"
              marginTop={majorScale(4)}
              marginBottom={majorScale(3)}
            ></Pane>
          )}
          {isWalletConnected && !isLoading && (
            <SignMessageButton
              label="Unlock shielded balance"
              message={SIGN_MESSAGE}
              // @ts-ignore
              type="submit"
              appearance="primary"
              size="large"
              width="-webkit-fill-available"
            />
          )}
          {isWalletConnected && isLoading && (
            <Button
              appearance="primary"
              size="large"
              width="-webkit-fill-available"
            >
              Unlocking...
            </Button>
          )}
        </form>

        <>{!isWalletConnected && <Wallet />}</>
      </Pane>
    </>
  );
};
