import { Paper, Text, Group, Stack, Divider, Anchor } from "@mantine/core";
import {
  IconArrowDown,
  IconArrowUp,
  IconArrowRight,
} from "@tabler/icons-react";
import { TOKEN_REGISTRY, UserIndexedTransaction } from "@lightprotocol/zk.js";
import { useTransactions } from "../../state/hooks/useTransactions";
import { Pagination } from "@mantine/core";

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
  const tokenCtx = TOKEN_REGISTRY.get("SOL")!;
  const parsedRelayerFee = parseAmount(relayerFee, tokenCtx);

  return (
    <Stack mt={"xl"} gap="xs" w={300} data-testid="TransactionCard">
      <Text size="xs" c="gray">
        {format(new Date(blockTime), "do MMM yyyy, HH:mm")}
      </Text>
      <Group justify="space-between">
        <Group gap={8}>
          {typeIcons[type]}
          <Text size="md">{type.toLowerCase()}</Text>
        </Group>
        <Stack gap={0}>
          <Group align="baseline" gap={4} justify="flex-end">
            <Text size="sm">{parseTxAmount(transaction)}</Text>
            {/* TODO: replace with actual MINT */}
            <Text size="sm">SOL</Text>
          </Group>
          <Text size="sm" c="gray">
            Fees paid: {parsedRelayerFee} SOL
          </Text>
        </Stack>
      </Group>
      <Anchor
        size="xs"
        c="blue"
        href={`https://explorer.solana.com/tx/${signature}?cluster=devnet`}
        target="_blank"
      >
        {signature.slice(0, 6) + "..." + signature.slice(-6)}
      </Anchor>
      <Divider my="xs" color="#f3f6f9" />
    </Stack>
  );
}

import { format } from "date-fns";
import { useState } from "react";
import { parseAmount, parseTxAmount } from "../../utils/parser";

export const Transactions = () => {
  const { transactions } = useTransactions();
  const [currentPage, setCurrentPage] = useState(1);
  const itemsPerPage = 4;

  const sortedTransactions = transactions!.sort(
    (a, b) => new Date(b.blockTime).getTime() - new Date(a.blockTime).getTime()
  );

  const currentPageTransactions = sortedTransactions.slice(
    (currentPage - 1) * itemsPerPage,
    currentPage * itemsPerPage
  );

  console.log(
    "TXS",
    currentPageTransactions?.map(
      (tx) => `${tx.publicAmountSol} and spl ${tx.publicAmountSpl}`
    )
  );

  return (
    <>
      <Paper p="md" radius="xs" withBorder role="region">
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
