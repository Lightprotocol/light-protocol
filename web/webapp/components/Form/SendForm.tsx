import React, { useCallback } from "react";
import { Box, Stack, Group, Button, Text, Paper, rem } from "@mantine/core";
import { useForm, UseFormReturnType } from "@mantine/form";
import { IconArrowRight, IconShieldDown } from "@tabler/icons-react";
import { TokenInput, SendRecipientInput } from "../Input";
import { FormValues } from ".";
import { useAction } from "../../state/hooks/useAction";
import { useSend } from "../../state/hooks/useSend";
import { useSendType } from "../../state/hooks/useSendType";
import { Chip } from "@mantine/core";

export interface SendFormValues extends FormValues {
  recipient: string;
}

export function SendForm() {
  const form: UseFormReturnType<SendFormValues> = useForm({
    initialValues: { amount: "", token: "SOL", recipient: "" },
  });
  const isDecompress = useSendType(form.values.recipient);
  const { transfer, decompress, loading } = useAction();
  const send = useSend();

  const handleSubmit = useCallback(
    async (values: SendFormValues) => {
      await send(values, isDecompress);
    },
    [decompress, transfer, isDecompress]
  );

  return (
    <Box w={"100%"} mx="auto">
      <form
        aria-disabled={loading}
        onSubmit={form.onSubmit(handleSubmit)}
        data-testid="send-form"
      >
        <TokenInput form={form} disabled={loading} />
        <SendRecipientInput form={form} />
        <Stack mt="md" gap={28}>
          {form.values.amount && form.values.recipient && (
            <Stack mt="xl" gap={8}>
              {isDecompress && (
                <Chip
                  icon={
                    <IconShieldDown
                      style={{ width: rem(16), height: rem(16) }}
                    />
                  }
                  variant="light"
                  size="xs"
                >
                  Decompress
                </Chip>
              )}
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
