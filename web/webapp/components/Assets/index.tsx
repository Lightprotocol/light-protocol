import React from "react";
import {
  Group,
  Paper,
  Table,
  Menu,
  Button,
  Title,
  Divider,
} from "@mantine/core";
import { modals } from "@mantine/modals";
import { useBalance } from "../../state/hooks/useBalance";
import { IconDotsVertical } from "@tabler/icons-react";
import { parseShieldedBalance } from "../../utils/parser";
import { useState } from "react";
import { ModalContent } from "../Modal";
export const Assets = () => {
  const { balance } = useBalance();
  const [opened, setOpened] = useState(false);
  const rows =
    balance &&
    Array.from(balance.keys()).map((token, index) => {
      const tokenBalance = balance.get(token);
      return tokenBalance ? (
        <Table.Tr key={index}>
          <Table.Td style={{ padding: "20px" }}>
            {parseShieldedBalance(tokenBalance).token}
          </Table.Td>
          <Table.Td style={{ padding: "20px" }}>
            {parseShieldedBalance(tokenBalance).balance}
          </Table.Td>
          <Table.Td style={{ padding: "20px" }}>
            <Group justify="flex-end">
              <Menu
                shadow="md"
                width={100}
                opened={opened}
                onChange={() => setOpened(!opened)}
              >
                <Menu.Target>
                  <Button
                    rightSection={<IconDotsVertical />}
                    size="compact-xs"
                    variant="secondary"
                  >
                    {/* <IconDotsVertical /> */}
                  </Button>
                </Menu.Target>
                <Menu.Dropdown>
                  <Menu.Item
                    onClick={() => {
                      modals.open({
                        withCloseButton: false,
                        overlayProps: { backgroundOpacity: 0.2 },
                        size: "sm",
                        radius: "lg",
                        children: <ModalContent initValue="shield" />,
                      });
                    }}
                  >
                    Shield
                  </Menu.Item>
                  <Menu.Item
                    onClick={() => {
                      modals.open({
                        withCloseButton: false,
                        overlayProps: { backgroundOpacity: 0.2 },
                        size: "sm",
                        radius: "lg",
                        children: <ModalContent initValue="send" />,
                      });
                    }}
                  >
                    Send
                  </Menu.Item>
                </Menu.Dropdown>
              </Menu>
            </Group>
          </Table.Td>
        </Table.Tr>
      ) : null;
    });

  return (
    <Paper radius="md" withBorder w="400px" role="region">
      <Paper style={{ width: "100%", padding: "20px" }}>
        <Title size="xs"> My Shielded Assets</Title>
      </Paper>
      <Divider></Divider>
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
