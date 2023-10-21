"use client";
import { useDisclosure } from "@mantine/hooks";
import {
  AppShell,
  Burger,
  Button,
  Group,
  Paper,
  Modal,
  Stack,
  SegmentedControl,
  Box,
  TextInput,
  Text,
  Title,
} from "@mantine/core";
import { NativeSelect, rem } from "@mantine/core";

import { useForm } from "@mantine/form";
import { usePathname, useRouter } from "next/navigation";
import { Navbar } from "../components/Navbar";
import { useState } from "react";
import { TokenPicker } from "../components/TokenPicker";
import { IconArrowRight } from "@tabler/icons-react";

import { useFocusTrap } from "@mantine/hooks";

export function TokenInput({ form }: { form: any }) {
  const focusTrapRef = useFocusTrap();

  const select = <TokenPicker form={form} />;

  return (
    <>
      <Stack py={"md"} gap={0}>
        <Group px={"md"} justify="flex-start" align="center">
          <Paper withBorder shadow="xs" radius={"xl"} px={"xs"}>
            <Text c="grey">MAX</Text>
          </Paper>
          {/* <Paper radius={"md"} px="xs">
            <u>0.00</u>
          </Paper> */}
        </Group>
        <TextInput
          ref={focusTrapRef}
          {...form.getInputProps("amount")}
          px={"md"}
          pb={"md"}
          variant="unstyled"
          size="xl"
          type="number"
          placeholder="0.00"
          rightSection={select}
          rightSectionWidth={"45%"}
          autoFocus={true}
          styles={{
            input: {
              fontSize: "40px",
            },
          }}
        />
      </Stack>
    </>
  );
}
export const ShieldForm = () => {
  const form = useForm({ initialValues: { amount: "", token: "SOL" } });

  return (
    <Box w={"100%"} mx="auto">
      <form onSubmit={form.onSubmit((values) => console.log(values))}>
        <TokenInput form={form} />
        <Stack mt="md" gap={28}>
          {form.values.amount && (
            <Stack mt="xl" gap={8}>
              <Group w="100%" px="20px" justify="space-between">
                <Text size="sm">To</Text>
                <Text size="sm">My account</Text>
              </Group>
              <Group w="100%" px="20px" justify="space-between">
                <Text size="sm">Network fee</Text>
                <Text size="sm">0.001 SOL</Text>
              </Group>
              <Group w="100%" px="20px" justify="space-between">
                <Text size="sm">Shield time</Text>
                <Text size="sm">~3s</Text>
              </Group>
            </Stack>
          )}
          <Button
            justify="space-between"
            fullWidth
            size="lg"
            radius="xl"
            type="submit"
            rightSection={<IconArrowRight />}
          >
            Shield now
          </Button>
        </Stack>
      </form>
    </Box>
  );
};
export const SendForm = () => {};

export const ShieldSendModal = () => {
  const [opened, { open, close }] = useDisclosure(true);
  const [value, setValue] = useState("shield");
  return (
    <>
      <Modal
        opened={opened}
        onClose={close}
        withCloseButton={false}
        overlayProps={{ backgroundOpacity: 0.2 }}
        size="sm"
        radius={"lg"}
        keepMounted={false}
      >
        <Stack>
          <Box px={"md"}>
            <SegmentedControl
              value={value}
              fullWidth
              color="#0066FF"
              onChange={setValue}
              radius={"xl"}
              data={[
                { label: "Shield", value: "shield" },
                { label: "Send", value: "send" },
              ]}
            />
          </Box>
          {value === "shield" ? <ShieldForm /> : <SendForm />}
        </Stack>
      </Modal>

      <Button radius={"xl"} onClick={open}>
        Shield & Send
      </Button>
    </>
  );
};

export default function Shell() {
  const [opened, { toggle }] = useDisclosure();
  const router = useRouter();
  const path = usePathname();
  return (
    <AppShell
      layout="alt"
      header={{ height: 60 }}
      navbar={{ width: 250, breakpoint: "sm", collapsed: { mobile: !opened } }}
      padding="md"
    >
      <Navbar router={router} path={path} />
      <AppShell.Header>
        <Group justify="space-between" h="100%" px="md" pl={"lg"}>
          <Burger opened={opened} onClick={toggle} hiddenFrom="sm" size="sm" />
          <Title size={"sm"}> My assets</Title>
          <Group>
            <ShieldSendModal />
            zk account wallet connection
          </Group>
        </Group>
      </AppShell.Header>
      <AppShell.Main>Main</AppShell.Main>
    </AppShell>
  );
}
