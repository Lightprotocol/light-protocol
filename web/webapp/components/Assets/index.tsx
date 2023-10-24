import { TokenUtxoBalance } from "@lightprotocol/zk.js";
import { Group, Paper, Table, Text } from "@mantine/core";
import { useBalance } from "../../state/hooks/useBalance";
import { IconDotsVertical } from "@tabler/icons-react";

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

export const Assets = () => {
  const { balance } = useBalance();

  const rows =
    balance &&
    Array.from(balance.keys()).map((token, index) => {
      const tokenBalance = balance.get(token);
      return tokenBalance ? (
        <Table.Tr key={index}>
          <Table.Td style={{ padding: "20px" }}>
            {parseBalance(tokenBalance).token}
          </Table.Td>
          <Table.Td style={{ padding: "20px" }}>
            {parseBalance(tokenBalance).balance}
          </Table.Td>
          <Table.Td style={{ padding: "20px" }}>
            <Group justify="flex-end">
              <IconDotsVertical />
            </Group>
          </Table.Td>
        </Table.Tr>
      ) : null;
    });

  return (
    <Paper radius="md" withBorder w="400px">
      <Table highlightOnHover>
        <Table.Thead>
          <Table.Tr>
            <Table.Th style={{ width: "30%", padding: "20px" }}>Name</Table.Th>
            <Table.Th style={{ width: "50%", padding: "20px" }}>
              Balance
            </Table.Th>
            <Table.Th style={{ padding: "20px" }}></Table.Th>
          </Table.Tr>
        </Table.Thead>
        <Table.Tbody>{rows}</Table.Tbody>
      </Table>
    </Paper>
  );
};
