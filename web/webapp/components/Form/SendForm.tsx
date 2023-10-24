import { Box, Stack, Group, Button, Text } from "@mantine/core";
import { useForm, UseFormReturnType } from "@mantine/form";
import { IconArrowRight } from "@tabler/icons-react";
import { TokenInput, SendRecipientInput } from "../Input";
import { FormValues } from ".";
// TODO: add global jotai state to synchronize the form values to add "select recipient" page

export interface SendFormValues extends FormValues {
  recipient: string;
}

export function SendForm() {
  const form: UseFormReturnType<SendFormValues> = useForm({
    initialValues: { amount: "", token: "SOL", recipient: "" },
  });

  return (
    <Box w={"100%"} mx="auto">
      <form
        onSubmit={form.onSubmit((values) => {
          console.log(values);
        })}
      >
        <TokenInput form={form} />

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
