//@ts-check
import { useEffect } from "react";
import "react-loading-skeleton/dist/skeleton.css";
import "../css/App.css";
import { Header } from "./Header";
import { Pane, majorScale, toaster } from "evergreen-ui";
import { RenderShieldedBalance } from "./RenderShieldedBalance";
import { DepositForm } from "./DepositForm";
import { FormWrapper } from "./FormWrapper";
import { WithdrawalForm } from "./WithdrawalForm";
import { MERGE_UTXOS_THRESHOLD, SHOW_ERROR_DURATION } from "../constants";
import { DepositLoading } from "./DepositLoading";
import { WithdrawalLoading } from "./WithdrawalLoading";
import { SetupDialog } from "./SetupDialog";
import { useConnection, useWallet } from "@solana/wallet-adapter-react";
import { Unlock } from "./Unlock";
import { Footer } from "./Footer";
import { ReclaimBalance } from "./ReclaimBalance";
import { Blocker } from "./Blocker";
import { useAtom } from "jotai";
import { isMobileAtom, setDeviceWidthAtom } from "../state/deviceAtoms";
import { errorAtom, resolveErrorAtom } from "../state/errorAtoms";
import {
  Action,
  cancelActionAtom,
  isInProgressAtom,
  navigationAtom,
} from "../state/navigationAtoms";
import { updatePriceOracleAtom } from "../state/priceOracleAtoms";
import { fetchedUserUtxosAtom } from "../state/userUtxoAtoms";
import {
  fetchLeavesAtom,
  fetchNullifiersAtom,
  fetchedTransactionsAtom,
  leavesAtom,
  nullifierAtom,
  transactionsAtom,
} from "../state/transactionsAtoms";
import {
  checkedIsBlockedAtom,
  isBlockedAtom,
  isLoggedInAtom,
  isRegisteredAtom,
  isWalletConnectedAtom,
  resetUserStateAtom,
  userKeypairsAtom,
  fetchedUserAccountAtom,
  isFetchingUserAccountAtom,
  connectWalletAtom,
} from "../state/userAtoms";
import {
  globalMountMessage,
  globalMergeUtxosMessage,
} from "../api/globalMessage";
import { verificationPdaAtom } from "../state/utilAtoms";
import {
  activeUnspentUtxosAtom,
  activeUserUtxosAtom,
} from "../state/activeAtoms";
import {
  publicBalancesAtom,
  burnerBalanceAtom,
  fetchedPublicBalancesAtom,
  addPublicBurnerBalanceAtom,
} from "../state/balancesAtoms";

/** comment next line in development! */
console.log = function () {};

export const Page = () => {
  const { publicKey, disconnect } = useWallet();
  const { connection } = useConnection();
  const [isMobile] = useAtom(isMobileAtom);
  const [lightError] = useAtom(errorAtom);
  const [userKeypairs] = useAtom(userKeypairsAtom);
  const [publicBalances] = useAtom(publicBalancesAtom);
  const [burnerBalance] = useAtom(burnerBalanceAtom);
  const [isLoggedIn] = useAtom(isLoggedInAtom);
  const [isRegistered] = useAtom(isRegisteredAtom);
  const [isWalletConnected] = useAtom(isWalletConnectedAtom);
  const [currentWallet] = useAtom(connectWalletAtom);
  const [isFetchingUserAccount] = useAtom(isFetchingUserAccountAtom);
  const [isBlocked] = useAtom(isBlockedAtom);
  const [navigationState] = useAtom(navigationAtom);
  const [isInProgress] = useAtom(isInProgressAtom);
  const [activeUnspentUtxos] = useAtom(activeUnspentUtxosAtom);
  const [verificationPda] = useAtom(verificationPdaAtom);
  const [_, setDeviceWidth] = useAtom(setDeviceWidthAtom);
  const [__, resolveError] = useAtom(resolveErrorAtom);
  const [___, cancelAction] = useAtom(cancelActionAtom);
  const [____, fetchAllTransactions] = useAtom(fetchedTransactionsAtom);
  const [_____, checkIsBlocked] = useAtom(checkedIsBlockedAtom);
  const [______, fetchPrices] = useAtom(updatePriceOracleAtom);
  const [_______, fetchPublicBalances] = useAtom(fetchedPublicBalancesAtom);
  const [________, fetchBurnerBalance] = useAtom(addPublicBurnerBalanceAtom);
  const [_________, logOut] = useAtom(resetUserStateAtom);
  const [__________, fetchUserUtxos] = useAtom(fetchedUserUtxosAtom);
  const [___________, fetchUserAccount] = useAtom(fetchedUserAccountAtom);
  const [_____________, connectWallet] = useAtom(connectWalletAtom);
  const [______________, fetchLeaves] = useAtom(fetchLeavesAtom);
  const [_______________, fetchNullifiers] = useAtom(fetchNullifiersAtom);
  const [leaves] = useAtom(leavesAtom);
  const [transactions] = useAtom(transactionsAtom);
  const [nullifiers] = useAtom(nullifierAtom);

  const updateMedia = () => {
    setDeviceWidth(window.innerWidth);
  };

  useEffect(() => {
    window.addEventListener("resize", updateMedia);
    return () => window.removeEventListener("resize", updateMedia);
  }, [window.innerWidth]);

  useEffect(() => {
    toaster.closeAll();
    globalMountMessage();
  }, []);

  useEffect(() => {
    if (activeUnspentUtxos.length > MERGE_UTXOS_THRESHOLD)
      globalMergeUtxosMessage(activeUnspentUtxos);
  }, [activeUnspentUtxos]);

  useEffect(() => {
    (async () => {
      if (publicBalances.length > 0) {
        // TODO: add cache to check if changes before fetching
        console.log("checkBlocked/ publicBal efx");
        await checkIsBlocked();
      }
    })();
  }, [publicBalances, checkIsBlocked]);

  useEffect(() => {
    (async () => {
      if (isWalletConnected) {
        console.log("fetchUserAccount efx (once)");
        await fetchUserAccount(connection);
      }
    })();
  }, [isWalletConnected, fetchUserAccount]); // isRegistered,

  useEffect(() => {
    (async () => {
      if (isWalletConnected && isLoggedIn && isRegistered) {
        console.log("+ fetchutxos efx (once)");
        await fetchBurnerBalance(connection);
        if (transactions.length === 0) {
          await fetchAllTransactions();
        }
        // const calls = [];
        if (leaves.dedupedSortedLeafAccountBytesSol.length === 0)
          // calls.push(fetchLeaves);
          await fetchLeaves();
        if (nullifiers.length === 0) await fetchNullifiers();
        // calls.push(fetchNullifiers);
        // await Promise.all(calls);
        await fetchUserUtxos();
      }
    })();
  }, [
    isLoggedIn,
    isRegistered,
    isWalletConnected,
    fetchUserUtxos,
    fetchLeaves,
    fetchAllTransactions,
    fetchNullifiers,
    fetchBurnerBalance,
  ]);

  useEffect(() => {
    (async () => {
      console.log("connectWallet efx (once)", publicKey);
      if (publicKey) {
        if (
          currentWallet &&
          currentWallet.toBase58() !== publicKey.toBase58()
        ) {
          logOut();
          return disconnect();
        }
        connectWallet(publicKey);
        fetchPublicBalances(connection); //
      } else if (!publicKey) {
        logOut();
      }
    })();
  }, [publicKey, fetchPublicBalances]);

  useEffect(() => {
    (async () => {
      console.log("prices efx (once?)");
      await fetchAllTransactions();
      await fetchPrices();
      await fetchLeaves();
      console.log(
        "fetchedLeaves",
        leaves.dedupedSortedLeafAccountBytesSol.length,
      );

      console.log("fetchedTransactions", transactions.length);
      await fetchNullifiers();

      console.log("fetchedNullifiers", nullifiers.length);
    })();
  }, [fetchPrices, fetchLeaves, fetchAllTransactions, fetchNullifiers]);

  useEffect(() => {
    if (lightError.isError) {
      toaster.closeAll();
      toaster.danger(lightError.message, {
        duration: SHOW_ERROR_DURATION,
      });
      resolveError();
      cancelAction();
    }
  }, [lightError, resolveError, cancelAction]);

  return (
    <Pane marginBottom="2vh">
      {isBlocked ? (
        <Blocker />
      ) : (
        <>
          <Header isPhone={isMobile} />
          <Pane
            marginTop={majorScale(3)}
            display="flex"
            flexDirection="column"
            className="App-header"
          >
            {isLoggedIn &&
              burnerBalance.amount >= 5000 &&
              connection &&
              !verificationPda && (
                <ReclaimBalance
                  connection={connection}
                  bkp={userKeypairs.burnerKeypair!}
                  verificationPda={verificationPda!}
                  publicKey={publicKey!}
                  burnerBalance={burnerBalance.amount}
                />
              )}
            {isLoggedIn && (
              <Pane>
                <Pane display="flex" flexDirection="column">
                  <RenderShieldedBalance />
                </Pane>
              </Pane>
            )}
            <Pane
              background="white"
              borderRadius={majorScale(3)}
              elevation={1}
              padding={majorScale(3)}
              paddingTop={majorScale(1)}
              minHeight={majorScale(20)}
              maxWidth={majorScale(50)}
              marginTop={majorScale(3)}
              minWidth={majorScale(50)}
            >
              {navigationState.action === Action.SHIELD && (
                <Pane display="flex" flexDirection="column">
                  {isLoggedIn && isInProgress ? (
                    <DepositLoading></DepositLoading>
                  ) : isRegistered && !isLoggedIn && !isFetchingUserAccount ? (
                    <Unlock />
                  ) : isWalletConnected &&
                    !isRegistered &&
                    !isFetchingUserAccount ? (
                    <SetupDialog />
                  ) : (
                    <FormWrapper>
                      <DepositForm />
                    </FormWrapper>
                  )}
                </Pane>
              )}
              {navigationState.action === Action.UNSHIELD && (
                <Pane display="flex" flexDirection="column">
                  {isLoggedIn && isInProgress ? (
                    <WithdrawalLoading></WithdrawalLoading>
                  ) : isRegistered && !isLoggedIn && !isFetchingUserAccount ? (
                    <Unlock />
                  ) : isWalletConnected &&
                    !isRegistered &&
                    !isFetchingUserAccount ? (
                    <SetupDialog />
                  ) : (
                    <FormWrapper>
                      <WithdrawalForm />
                    </FormWrapper>
                  )}
                </Pane>
              )}
            </Pane>
            <Footer />
          </Pane>
        </>
      )}
    </Pane>
  );
};
