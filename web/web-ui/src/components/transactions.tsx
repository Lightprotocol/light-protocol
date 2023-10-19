import { ArrowDownIcon, ArrowUpIcon, ArrowRightIcon } from "@chakra-ui/icons";
import { Box, Card, Icon, Link, Text } from "@chakra-ui/react";
import { IndexedTransaction } from "@lightprotocol/zk.js";

const parseTxAmount = (tx: IndexedTransaction) => {
  // TODO: consider decimals of actual mint
  return tx.publicAmountSpl ? tx.publicAmountSpl : tx.publicAmountSol;
};
function TransactionCard({ transaction }: { transaction: IndexedTransaction }) {
  const { type, blockTime, signature, relayerFee } = transaction;
  const typeIcons = {
    SHIELD: <ArrowDownIcon />,
    UNSHIELD: <ArrowUpIcon />,
    TRANSFER: <ArrowRightIcon />,
  };

  return (
    <Card p={4} borderRadius="md" boxShadow="md">
      <Box display="flex" flexDirection="column" alignItems="start">
        <Text fontSize="sm" color="gray.500">
          {new Date(blockTime).toLocaleString()}
        </Text>
        <Text fontSize="lg" fontWeight="bold">
          {type}
        </Text>
        <Icon boxSize={6}>{typeIcons[type]}</Icon>
        <Link
          fontSize="sm"
          color="blue.500"
          href={`https://explorer.solana.com/${signature}`}
        >
          {signature.slice(0, 6) + "..." + signature.slice(-6)}
        </Link>
      </Box>
      <Box mt={4} display="flex" flexDirection="column" alignItems="start">
        <Text fontSize="md" fontWeight="bold">
          PLACEHOLDER MINT
        </Text>
        <Text fontSize="lg">{parseTxAmount(transaction)}</Text>
        <Text fontSize="sm" color="gray.500">
          Fees paid: {relayerFee}
        </Text>
      </Box>
    </Card>
  );
}

export const Transactions = ({
  transactions,
}: {
  transactions: IndexedTransaction[];
}) => {
  return (
    <Box
      display="flex"
      flexDirection="column"
      alignItems="center"
      justifyContent="center"
      p={4}
    >
      {transactions.map((transaction: IndexedTransaction, index: number) => (
        <TransactionCard key={index} transaction={transaction} />
      ))}
    </Box>
  );
};
