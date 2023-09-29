import React from "react";
import { Pane, Heading, Spinner } from "evergreen-ui";
import { Wallet } from "./Wallet";
import { TransactionHistoryDialog } from "./TransactionHistoryDialog";
import Image from "../assets/svg/LogoPrimary.png";
import { sleep } from "../util/helpers";
import { useAtom } from "jotai";
import { transactionsAtom } from "../state/transactionsAtoms";
import { isLoggedInAtom } from "../state/userAtoms";

export const Header = ({ counter = 1, isPhone = true }) => {
  const [transactions] = useAtom(transactionsAtom);
  const [showTransactionHistoryDialog, setShowTransactionHistoryDialog] =
    React.useState(false);
  const [isLoggedIn] = useAtom(isLoggedInAtom);
  let transactionsLoaded = transactions.length > 0;
  return (
    <Pane
      paddingLeft="1em"
      paddingRight="1em"
      width="-webkit-fill-available"
      display="flex"
      justifyContent="space-between"
      alignItems="center"
      paddingTop="1em"
      border="1px solid rgba(0, 0, 0, .066)"
      borderLeft="none"
      borderRight="none"
      borderTop="none"
      borderBottomWidth="thin"
      paddingBottom="1em"
    >
      <Pane display="flex" alignItems="center">
        <img src={Image} width="190px" height="" />
        {counter !== 0 && !isPhone && (
          <Pane
            display="flex"
            marginLeft="8px"
            alignItems="start"
            justifyContent="center"
            flexDirection="column"
          >
            <Heading
              color="#8f95b2

            "
              size={500}
            >
              Protects your privacy
            </Heading>

            {isLoggedIn && (
              <Heading
                color={transactionsLoaded ? "#fff" : "#8f95b2"}
                background={transactionsLoaded ? "#06f" : undefined}
                marginTop="4px"
                size={300}
                border={
                  transactionsLoaded
                    ? "1.66px solid #06f"
                    : "1.66px solid #8f95b2"
                }
                paddingLeft="10px"
                paddingRight="10px"
                paddingTop="6px"
                paddingBottom="6px"
                borderRadius="24px"
                cursor="pointer"
                onClick={async () => {
                  await sleep(40);
                  setShowTransactionHistoryDialog(true);
                }}
              >
                {showTransactionHistoryDialog ? (
                  <Pane
                    display="flex"
                    alignItems="center"
                    justifyContent="center"
                  >
                    <Heading
                      size={300}
                      color={transactionsLoaded ? "#fff" : "#8f95b2"}
                      // marginRight="4px"
                    >
                      My Transactions
                    </Heading>
                    {/* <Spinner size={14} /> */}
                  </Pane>
                ) : (
                  "My Transactions"
                )}
              </Heading>
            )}
            <TransactionHistoryDialog
              showModal={showTransactionHistoryDialog}
              setShowModal={setShowTransactionHistoryDialog}
            />
          </Pane>
        )}
      </Pane>
      <Pane data-private marginLeft="1em">
        <Wallet />
      </Pane>
    </Pane>
  );
};
