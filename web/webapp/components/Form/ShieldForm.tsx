import React, { useState } from "react";
import { Box, Stack, Group, Button, Text } from "@mantine/core";
import { useForm, UseFormReturnType } from "@mantine/form";
import { IconArrowRight } from "@tabler/icons-react";
import { TokenInput } from "../Input";
import { FormValues } from ".";
import { useAction } from "../../state/hooks/useAction";
import { notifications } from "@mantine/notifications";
import { modals } from "@mantine/modals";

export interface ShieldFormValues extends FormValues {}

export const ShieldForm = () => {
  const form: UseFormReturnType<ShieldFormValues> = useForm({
    initialValues: { amount: "", token: "SOL" },
  });

  const { shield } = useAction();
  const [loading, setLoading] = useState(false);

  return (
    <Box w={"100%"} mx="auto">
      <form
        onSubmit={form.onSubmit(async (values) => {
          console.log(values);
          setLoading(true);
          console.log("shielding");
          notifications.show({
            title: "Sending transaction",
            message: "",
            color: "blue",
            autoClose: 4000,
          });

          //TODO: adapt such that we can build and send and confirm separately for notifs.
          try {
            await shield({
              token: values.token,
              publicAmountSol:
                values.token === "SOL" ? values.amount : undefined,
              publicAmountSpl:
                values.token !== "SOL" ? values.amount : undefined,
            });
            console.log("shielded");
            notifications.show({
              title: "Transaction successful",
              message: "",
              color: "green",
              autoClose: 3000,
            });
            window.setTimeout(() => {
              modals.closeAll(); // this is quite optimistic
            }, 1500);
          } catch (e) {
            console.error(e);
            throw e;
          } finally {
            window.setTimeout(() => {
              setLoading(false);
            }, 1500);
          }
        })}
      >
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
            loading={loading}
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
