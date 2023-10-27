import React, { useCallback, useEffect } from "react";
import { Box, Stack, Group, Button, Text } from "@mantine/core";
import { useForm, UseFormReturnType } from "@mantine/form";
import { IconArrowRight } from "@tabler/icons-react";
import { TokenInput, SendRecipientInput } from "../Input";
import { FormValues } from ".";
import { useAction } from "../../state/hooks/useAction";
import { notifications } from "@mantine/notifications";
import { modals } from "@mantine/modals";
import { useSend } from "../../state/hooks/useSend";
import { useSendType } from "../../state/hooks/useSendType";

export interface SendFormValues extends FormValues {
  recipient: string;
}

export function SendForm() {
  const form: UseFormReturnType<SendFormValues> = useForm({
    initialValues: { amount: "", token: "SOL", recipient: "" },
  });
  const isUnshield = useSendType(form.values.recipient);
  const { transfer, unshield, loading } = useAction();
  const send = useSend();

  const handleSubmit = useCallback(
    async (values: SendFormValues) => {
      await send(values, isUnshield);
    },
    [unshield, transfer]
  );

  useEffect(() => {
    if (loading) {
      notifications.show({
        title: `Sending ${form.values.token}`,
        message: "",
        color: "blue",
        autoClose: 5000,
      });
    } else {
      notifications.show({
        title: "Transfer successful",
        message: "",
        color: "green",
        autoClose: 3000,
      });
      modals.closeAll();
    }
  }, [loading]);

  return (
    <Box w={"100%"} mx="auto">
      <form aria-disabled={loading} onSubmit={form.onSubmit(handleSubmit)}>
        <TokenInput form={form} disabled={loading} />
        <SendRecipientInput form={form} />
        <Stack mt="md" gap={28}>
          {form.values.amount && form.values.recipient && (
            <Stack mt="xl" gap={8}>
              <Group w="100%" px="20px" justify="space-between">
                <Text size="sm">Network fee</Text>
                <Text size="sm">0.001 SOL</Text>
              </Group>
              <Group w="100%" px="20px" justify="space-between">
                <Text size="sm">Send time</Text>
                <Text size="sm">~3s</Text>
              </Group>
            </Stack>
          )}
          <Button
            justify="space-between"
            fullWidth
            size="lg"
            loading={loading}
            radius="xl"
            type="submit"
            rightSection={<IconArrowRight />}
          >
            Send now
          </Button>
        </Stack>
      </form>
    </Box>
  );
}
