import { TextInput } from "@mantine/core";

export function SendRecipientInput({ form }: { form: any }) {
  return (
    <TextInput
      {...form.getInputProps("recipient")}
      px={"md"}
      pb={"md"}
      data-testid="recipient-input"
      variant="unstyled"
      size="lg"
      type="string"
      label="To"
      placeholder="Paste recipient"
      autoFocus={true}
      styles={{
        label: {
          fontSize: "14px",
          marginBottom: "0px",
          color: "grey",
        },
      }}
    />
  );
}
