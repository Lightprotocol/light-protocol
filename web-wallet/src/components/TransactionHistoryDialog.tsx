import React, { useEffect, useState } from "react";
import { Dialog, Pane, Heading } from "evergreen-ui";
import { TransactionHistoryRow } from "./TransactionHistoryRow";
import { useAtom } from "jotai";
import {
  fetchedTransactionsAtom,
  groupedTransactionsAtom,
  userTransactionsAtom,
} from "../state/transactionsAtoms";

export function TransactionHistoryDialog({
  showModal,
  setShowModal,
}: {
  showModal: boolean;
  setShowModal: Function;
}) {
  const [now, setNow] = useState(new Date().getTime());
  const [userTransactions] = useAtom(userTransactionsAtom);
  const [groupedTransactions] = useAtom(groupedTransactionsAtom);
  const [____, fetchAllTransactions] = useAtom(fetchedTransactionsAtom);
  const [copied, setCopied] = useState(false);
  const [clicked, setClicked] = useState(false);

  React.useEffect(() => {
    if (copied) {
      // @ts-ignore //FIXME: copy doesnt work
      navigator.clipboard.writeText(copied.copied);
      setTimeout(() => {
        setCopied(false);
      }, 1666);
    }
  });
  const ReloadButton = () => {
    return (
      <>
        <Pane
          display="flex"
          borderRadius="4px"
          marginTop="8px"
          marginBottom="16px"
          minWidth="8vw"
          onClick={() => {
            (async () => {
              setClicked(true);
              setNow(new Date().getTime());
              await fetchAllTransactions();
              setTimeout(() => {
                setClicked(false);
              }, 2066);
            })();
          }}
          cursor="pointer"
        >
          {!clicked ? (
            <Heading textDecoration="underline" size={400}>
              Refresh
            </Heading>
          ) : (
            <Heading textDecoration="none" size={400}>
              Refreshing ...
            </Heading>
          )}
        </Pane>
      </>
    );
  };
  return (
    <Dialog
      isShown={showModal}
      shouldCloseOnOverlayClick={true}
      onCloseComplete={() => setShowModal(false)}
      hasClose={false}
      hasFooter={false}
      width="max-content"
    >
      <Pane
        display="flex"
        flexDirection="column"
        padding="2em"
        paddingBottom="2em"
        paddingTop="0em"
      >
        <Pane>
          <Heading marginBottom="16px" size={900}>
            Your Light transaction history
          </Heading>
          <Heading marginBottom="4px" size={400}>
            Only you can view your Light transaction history. <br></br>Your
            history is derived locally in your browser using on-chain data +
            your Light secret keys that only you own. <br />
            There's no deterministic way of re-creating your full history
            (shields + unshields) without having access to your secret keys.{" "}
            <br />
            Light doesn't collect and store any of your secret data either.
          </Heading>
          <Heading>
            It may take up to 30 seconds for your latest transactions to appear
            after you press refresh.
          </Heading>

          <ReloadButton />

          <Pane>
            {groupedTransactions.length === 0 ? (
              <Pane alignItems="center" display="flex" width="42em">
                <Heading color="grey">
                  Start shielding/unshielding to view your transaction history
                  here.
                </Heading>
              </Pane>
            ) : (
              groupedTransactions.map((batch, index) =>
                batch.name === "last60Minutes" || batch.name === "older"
                  ? batch.transactions
                      .sort((a, b) => b.blockTime - a.blockTime)
                      .map((tx, i) => (
                        // TODO: account for 1+ year cases separately at some point
                        <Pane
                          display="flex"
                          flexDirection="column"
                          gap="8px"
                          marginBottom="16px"
                        >
                          <Heading size={100}>
                            {Math.floor((now - tx.blockTime) / 1000 / 60) === 0
                              ? "Just now"
                              : Math.floor((now - tx.blockTime) / 1000 / 60) ===
                                1
                              ? Math.floor((now - tx.blockTime) / 1000 / 60) +
                                ` Minute ago`
                              : Math.floor((now - tx.blockTime) / 1000 / 60) +
                                ` Minutes ago`}{" "}
                          </Heading>
                          <TransactionHistoryRow
                            tx={tx}
                            // @ts-ignore
                            copied={copied}
                            setCopied={setCopied}
                            index={i}
                          />
                        </Pane>
                      ))
                  : batch.transactions
                      .sort((a, b) => a.t - b.t) // distance of time instead of time
                      .map(
                        (tx, i) =>
                          tx.tx.length > 0 && (
                            <Pane
                              display="flex"
                              flexDirection="column"
                              gap="8px"
                              marginBottom="16px"
                            >
                              {batch.name === "last24HoursByHour" && (
                                <Heading size={100}>
                                  {tx.t === 1
                                    ? "1 hour ago"
                                    : `${tx.t} hours ago`}
                                </Heading>
                              )}
                              {batch.name === "last30DaysByDay" && (
                                <Heading size={100}>
                                  {tx.t === 1
                                    ? "1 day ago"
                                    : `${tx.t} days ago`}
                                </Heading>
                              )}
                              {batch.name === "lastYearByMonth" && (
                                <Heading size={100}>
                                  {tx.t === 1
                                    ? "1 month ago"
                                    : `${tx.t} months ago`}
                                </Heading>
                              )}

                              {tx.tx
                                .sort((a, b) => b.blockTime - a.blockTime)
                                .map((tx, i) => (
                                  <TransactionHistoryRow
                                    tx={tx}
                                    copied={{ copied: copied, index: i }}
                                    setCopied={setCopied}
                                    index={i}
                                  />
                                ))}
                            </Pane>
                          ),
                      ),
              )
            )}
          </Pane>
        </Pane>
      </Pane>
    </Dialog>
  );
}
