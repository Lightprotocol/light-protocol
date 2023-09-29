import React from "react";
import {
  Pane,
  Heading,
  ArrowUpIcon,
  ArrowDownIcon,
  Avatar,
  CircleArrowDownIcon,
  DuplicateIcon,
  Icon,
  ArrowTopRightIcon,
  majorScale,
  Link,
} from "evergreen-ui";
import { DECIMALS_SOL, DECIMALS, Token } from "../constants.js";

import { PDFDownloadLink } from "@react-pdf/renderer";
import { useWallet } from "@solana/wallet-adapter-react";
import { ProofDocument } from "./ProofDocument";
import { getTransactionAndUtxoGroups } from "../util/transactionHistory";
import { useAtom } from "jotai";
import { userTransactionsAtom } from "../state/transactionsAtoms";
import { spentUtxosAtom, userUtxosAtom } from "../state/userUtxoAtoms";

export function TransactionHistoryRow({
  tx,
  copied,
  setCopied,
  index,
}: {
  tx: any;
  copied: { copied: boolean; index: number };
  setCopied: Function;
  index: number;
}) {
  const [userTransactions] = useAtom(userTransactionsAtom);
  const [spentUtxos] = useAtom(spentUtxosAtom);
  const [transactionAndUtxoGroups, setTransactionAndUtxoGroups] =
    React.useState([]);
  const { publicKey } = useWallet();

  const options = { month: "long" };
  const getMonth = (month: any) =>
    // @ts-ignore

    new Intl.DateTimeFormat("en-US", options).format(month);

  React.useEffect(() => {
    (async () => {
      let txg = await getTransactionAndUtxoGroups(
        userTransactions,
        spentUtxos,
        tx,
      );
      setTransactionAndUtxoGroups(txg);
    })();
  }, []);

  let decimals = tx.token === Token.SOL ? DECIMALS_SOL : DECIMALS;
  return (
    <Pane
      marginBottom="24px"
      display="flex"
      justifyContent="space-between"
      alignItems="center"
    >
      <Pane alignItems="center" display="flex" width="42em">
        <Pane display="flex" flexDirection="column">
          <Pane display="flex" alignItems="center">
            {tx.type === "shield" && (
              <Icon icon={ArrowUpIcon} color="success" marginRight="8px" />
            )}
            {tx.type === "unshield" && (
              <Icon icon={ArrowDownIcon} color="info" marginRight="8px" />
            )}
            <Heading size={400} width="9em" marginRight={majorScale(4)}>
              {tx.type === "shield" ? "Shield" : "Unshield"}
            </Heading>
          </Pane>
          <Pane display="flex" alignItems="center" width="max-content">
            <Heading size={100} color="#8f95b2" marginRight={majorScale(4)}>
              {new Date(tx.blockTime).getHours() +
                ":" +
                new Date(tx.blockTime).getMinutes() +
                " • " +
                new Date(tx.blockTime).getDate() +
                "th " +
                getMonth(new Date(tx.blockTime))}
            </Heading>
          </Pane>
        </Pane>
        <Heading
          display="flex"
          alignItems="center"
          size={400}
          width="6em"
          marginRight={majorScale(4)}
        >
          {tx.type === "shield"
            ? "+" +
              Math.round((tx.amount / decimals + Number.EPSILON) * 1000) / 1000
            : "-" +
              Math.round((tx.amount / decimals + Number.EPSILON) * 1000) / 1000}
          <Pane marginLeft="8px" display="flex" alignItems="center">
            {tx.token === Token.SOL ? (
              <Avatar
                src="https://raw.githubusercontent.com/solana-labs/token-list/main/assets/mainnet/So11111111111111111111111111111111111111112/logo.png"
                name="Solana Icon"
                size={18}
                marginRight="4px"
              />
            ) : tx.token === Token.USDC ? (
              <Avatar
                src="https://raw.githubusercontent.com/solana-labs/token-list/main/assets/mainnet/EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v/logo.png"
                name="USDC Icon"
                size={18}
                marginRight="4px"
              />
            ) : (
              ""
            )}
          </Pane>
        </Heading>

        <Heading size={400} width="12em" marginRight={majorScale(4)}>
          {tx.type === "unshield" && (
            <Pane display="flex" gap="4px" alignItems="center">
              <Heading size={400}>to</Heading>

              <Pane
                paddingX="4px"
                paddingY="2px"
                borderRadius="6px"
                border="0.66px solid #000"
                onClick={() => setCopied({ copied: tx.to, index })}
                cursor="pointer"
                display="flex"
                gap="4px"
                alignItems="center"
              >
                <Heading size={300}>
                  {copied.copied === tx.to && copied.index === index
                    ? "Copied!"
                    : tx.to?.slice(0, 5) + "..." + tx.to?.slice(-5)}
                </Heading>
                {copied && copied.index === index ? (
                  ""
                ) : (
                  <Icon icon={DuplicateIcon} color="#000" size={12} />
                )}
              </Pane>
            </Pane>
          )}
        </Heading>
        <Link
          target="_blank"
          textDecoration="none"
          href={`https://explorer.solana.com/tx/${tx.signature}`}
        >
          <Pane
            paddingX="4px"
            paddingY="2px"
            borderRadius="6px"
            border="0.66px solid #000"
            textDecoration="none"
            display="flex"
            gap="4px"
            alignItems="center"
          >
            <Heading size={300}>Explorer</Heading>
            <Icon icon={ArrowTopRightIcon} color="#000" size={12} />
          </Pane>
        </Link>
      </Pane>
      {tx && tx.type === "unshield" && transactionAndUtxoGroups.length > 0 && (
        <Pane
          paddingX="4px"
          paddingY="2px"
          borderRadius="6px"
          border="0.66px solid #000"
          display="flex"
          gap="4px"
          alignItems="center"
        >
          <PDFDownloadLink
            document={
              <ProofDocument
                data={transactionAndUtxoGroups}
                tx={tx}
                publicKey={publicKey?.toBase58()}
                activeToken={tx.token}
              />
            }
            style={{ textDecoration: "none" }}
            fileName={`proof_of_origin_of_funds_for_${tx.to}.pdf`}
          >
            {({ loading }) =>
              loading ? (
                <Heading size={300}> Proof of origin</Heading>
              ) : (
                <Heading size={300}> Proof of origin</Heading>
              )
            }
          </PDFDownloadLink>
          <Icon icon={CircleArrowDownIcon} color="#000" size={14} />
        </Pane>
      )}
    </Pane>
  );
}
