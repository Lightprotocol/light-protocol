import { Box, Stack, Group, Button, Text } from "@mantine/core";
import { useForm, UseFormReturnType } from "@mantine/form";
import { IconArrowRight } from "@tabler/icons-react";
import { TokenInput, SendRecipientInput } from "../Input";
import { FormValues } from ".";
import { useAction } from "../../state/hooks/useAction";
import { useEffect, useState } from "react";
import { PublicKey } from "@solana/web3.js";

export interface SendFormValues extends FormValues {
  recipient: string;
}

const isSolanaPublicKey = (string: string): boolean => {
  try {
    if (PublicKey.isOnCurve(string)) {
      new PublicKey(string);
      return true;
    }
    console.log("!isOnCurve");
    return false;
  } catch (err) {
    console.log("!Pubkey");
    return false;
  }
};

export function SendForm() {
  const [isUnshield, setIsUnshield] = useState(false);

  const form: UseFormReturnType<SendFormValues> = useForm({
    initialValues: { amount: "", token: "SOL", recipient: "" },
  });

  useEffect(() => {
    if (form.values.recipient) {
      setIsUnshield(isSolanaPublicKey(form.values.recipient));
    }
  }, [form.values.recipient]);

  const { transfer, unshield } = useAction();

  return (
    <Box w={"100%"} mx="auto">
      <form
        onSubmit={form.onSubmit((values) => {
          console.log(values);
          (async () => {
            if (isUnshield) {
              console.log("unshielding");
              await unshield({
                token: values.token,
                recipient: new PublicKey(values.recipient),
                publicAmountSol:
                  values.token === "SOL" ? values.amount : undefined,
                publicAmountSpl:
                  values.token !== "SOL" ? values.amount : undefined,
              });
            } else {
              console.log("transferring");

              await transfer({
                token: values.token,
                recipient: values.recipient,
                amountSol: values.token === "SOL" ? values.amount : undefined,
                amountSpl: values.token === "SOL" ? undefined : values.amount,
              });
            }
            console.log("done");
          })();
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
