import React, { useCallback } from "react";
import { Box, Stack, Group, Button, Text } from "@mantine/core";
import { useForm, UseFormReturnType } from "@mantine/form";
import { IconArrowRight } from "@tabler/icons-react";
import { TokenInput } from "../Input";
import { FormValues } from ".";
import { useAction } from "../../state/hooks/useAction";

export interface ShieldFormValues extends FormValues {}

export const ShieldForm = () => {
  const form: UseFormReturnType<ShieldFormValues> = useForm({
    initialValues: { amount: "", token: "SOL" },
  });
  const { shield, loading } = useAction();

  const handleSubmit = useCallback(
    async (values: ShieldFormValues) => {
      try {
        await shield({
          token: values.token,
          publicAmountSol: values.token === "SOL" ? values.amount : undefined,
          publicAmountSpl: values.token !== "SOL" ? values.amount : undefined,
        });
      } catch (e) {
        console.error(e);
        throw e;
      }
    },
    [shield],
  );

  return (
    <Box w={"100%"} mx="auto">
      <form aria-disabled={loading} onSubmit={form.onSubmit(handleSubmit)}>
        <TokenInput form={form} disabled={loading} />
        <Stack mt="md" gap={28}>
          {form.values.amount && (
            <Stack mt="xl" gap={8}>
              <Group w="100%" px="20px" justify="space-between">
                <Text size="sm">To</Text>
                <Text size="sm">My account</Text>
              </Group>
              <Group w="100%" px="20px" justify="space-between">
                <Text size="sm">Network fee</Text>
                {/* TODO: calculate the actual value from rpcInfo */}
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
