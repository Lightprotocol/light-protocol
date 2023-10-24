import { Paper, Text, Group, Stack, NavLink, Divider } from "@mantine/core";
import {
  IconArrowDown,
  IconArrowUp,
  IconArrowRight,
} from "@tabler/icons-react";
import {
  IndexedTransaction,
  UserIndexedTransaction,
} from "@lightprotocol/zk.js";
import { useTransactions } from "../../state/hooks/useTransactions";
import { Pagination } from "@mantine/core";
const parseTxAmount = (tx: UserIndexedTransaction) => {
  // TODO: consider decimals of actual mint
  //   console.log("tx", tx.publicAmountSpl, tx.publicAmountSol);
  return tx.publicAmountSpl ? tx.publicAmountSpl : tx.publicAmountSol;
};

function TransactionCard({
  transaction,
}: {
  transaction: UserIndexedTransaction;
}) {
  const { type, blockTime, signature, relayerFee } = transaction;
  const typeIcons = {
    SHIELD: <IconArrowDown color="green" size={20} />,
    UNSHIELD: <IconArrowUp color="red" />,
    TRANSFER: <IconArrowRight color="blue" />,
  };

  return (
    <Stack mt={"xl"} gap="xs" w={300}>
      <Text size="xs" c="gray">
        {format(new Date(blockTime), "do MMM yyyy, HH:mm")}
      </Text>
      <Group justify="space-between">
        <Group gap={8}>
          {typeIcons[type]}
          <Text size="md">{type.toLowerCase()}</Text>
        </Group>
        {/* <NavLink color="blue" href={`https://explorer.solana.com/${signature}`}>
        {signature.slice(0, 6) + "..." + signature.slice(-6)}
      </NavLink> */}
        <Stack gap={0}>
          <Group align="baseline" justify="flex-end">
            <Text size="sm">{parseTxAmount(transaction).toString()}</Text>
            <Text size="sm">SOL</Text>
          </Group>
          <Text size="sm" c="gray">
            Fees paid: {relayerFee.toString()} SOL
          </Text>
        </Stack>
      </Group>
      <Divider my="xs" color="#f3f6f9" />
    </Stack>
  );
}

import { format } from "date-fns";
import { useState } from "react";

export const Transactions = () => {
  const { transactions } = useTransactions();
  const [currentPage, setCurrentPage] = useState(1);
  const itemsPerPage = 4;

  // Sort the transactions
  const sortedTransactions = transactions!.sort(
    (a, b) => new Date(b.blockTime).getTime() - new Date(a.blockTime).getTime()
  );

  // Get the transactions for the current page
  const currentPageTransactions = sortedTransactions.slice(
    (currentPage - 1) * itemsPerPage,
    currentPage * itemsPerPage
  );

  return (
    <>
      <Paper p="md" radius="xs" withBorder>
        <Stack align="center" justify="center" gap="md">
          {currentPageTransactions.map(
            (transaction: UserIndexedTransaction, index: number) => (
              <TransactionCard key={index} transaction={transaction} />
            )
          )}
          <Pagination
            p={6}
            mt={8}
            total={Math.ceil(sortedTransactions.length / itemsPerPage)}
            value={currentPage}
            onChange={setCurrentPage}
          />
        </Stack>
      </Paper>
    </>
  );
};
