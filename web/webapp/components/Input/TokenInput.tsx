import { Stack, Group, Paper, TextInput, Text } from "@mantine/core";
import { useFocusTrap } from "@mantine/hooks";
import { TokenPicker } from "../TokenPicker";
import { UseFormReturnType } from "@mantine/form";
import { FormValues } from "../Form";

export function TokenInput<T extends FormValues>({
  form,
  disabled,
}: {
  form: UseFormReturnType<T>;
  disabled: boolean;
}) {
  const focusTrapRef = useFocusTrap();

  const select = <TokenPicker form={form} />;

  return (
    <>
      <Stack py={"md"} gap={0}>
        <Group px={"md"} justify="flex-start" align="center">
          <Paper withBorder shadow="xs" radius={"xl"} px={"xs"}>
            <Text c="grey">MAX</Text>
          </Paper>
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
          disabled={disabled}
        />
      </Stack>
    </>
  );
}
