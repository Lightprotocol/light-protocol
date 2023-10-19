"use client";
import React from "react";
import { Navbar } from "@/containers/navbar";
import { useBalance } from "@/state/hooks/useBalance";
import { BalanceBox } from "@/components/balance";
import { Transactions } from "@/components/transactions";
import { useTransactions } from "@/state/hooks/useTransactions";

export default function Page() {
  const { balance } = useBalance();
  const { transactions } = useTransactions();

  return (
    <>
      <Navbar />
      <Transactions transactions={transactions} />
      <BalanceBox balance={balance} />
    </>
  );
}
