"use client";
import React from "react";

/**
 * main page
 * to start off, will render: navbar (logo, shield/send button), (balances table), (shield btn -)
 *
 *
 * - header
 * -
 *
 */
import { Navbar } from "@/containers/navbar";
import { useBalance } from "@/state/hooks/useBalance";

import { IndexedTransaction, TokenUtxoBalance } from "@lightprotocol/zk.js";

function parseBalance(tokenBalance: TokenUtxoBalance) {
  let _token = tokenBalance.tokenData.symbol;
  let balance =
    _token === "SOL"
      ? tokenBalance.totalBalanceSol.toString()
      : tokenBalance.totalBalanceSpl.toString();
  let utxoNumber = tokenBalance.utxos.size;

  return {
    token: _token,
    balance: balance,
    utxos: utxoNumber,
  };
}

import { Box, Text } from "@chakra-ui/react";

import { Card, Icon, Link } from "@chakra-ui/react";
import { ArrowDownIcon, ArrowUpIcon, ArrowRightIcon } from "@chakra-ui/icons";

const TransactionCard = ({
  transaction,
}: {
  transaction: IndexedTransaction;
}) => {
  const {
    type,
    blockTime,
    signature,
    relayerFee,
    publicAmountSol,
    publicAmountSpl,
    toSpl,
    fromSpl,
  } = transaction;
  const typeIcons = {
    SHIELD: <ArrowDownIcon />,
    UNSHIELD: <ArrowUpIcon />,
    TRANSFER: <ArrowRightIcon />,
  };

  return (
    <Card className="transaction-card modern-look">
      <Box className="transaction-info">
        <Text className="transaction-date">
          {new Date(blockTime).toLocaleString()}
        </Text>
        <Text className="transaction-type">{type}</Text>
        <Icon className="transaction-icon">{typeIcons[type]}</Icon>
        <Link
          className="transaction-hash"
          href={`https://mock.com/${signature}`}
          isTruncated
        >
          {signature}
        </Link>
      </Box>
      <Box className="transaction-details">
        <Text className="transaction-token">{token}</Text>
        <Text className="transaction-amount">{amount}</Text>
        <Text className="transaction-fees">{fees}</Text>
      </Box>
    </Card>
  );
};

const Transactions = ({ transactions }) => {
  return (
    <Box className="transactions">
      {transactions.map((transaction, index) => (
        <TransactionCard key={index} transaction={transaction} />
      ))}
    </Box>
  );
};

export default function Page() {
  const { balance } = useBalance();

  return (
    <>
      <Navbar />
      {/* <Transactions /> */}
      <BalanceBox balance={balance} />
    </>
  );
}
