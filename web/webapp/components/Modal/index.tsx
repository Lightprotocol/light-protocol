import { Stack, Box, SegmentedControl, Button } from "@mantine/core";
import { modals } from "@mantine/modals";
import { useState, SetStateAction } from "react";
import { ShieldForm, SendForm } from "../Form";

export const ModalContent = ({
  initValue = "compress",
}: {
  initValue?: string; // TODO: enforce strict type checking
}) => {
  const [value, setValue] = useState(initValue);
  return (
    <Stack data-testid="compress-send-modal">
      <Box px={"md"}>
        <SegmentedControl
          data-testid="compress-send-control"
          value={value}
          fullWidth
          color="#0066FF"
          onChange={setValue}
          radius={"xl"}
          data={[
            { label: "Compress", value: "compress" },
            { label: "Send", value: "send" },
          ]}
        />
      </Box>
      {value === "compress" ? <ShieldForm /> : <SendForm />}
    </Stack>
  );
};

export const ShieldSendModal = () => {
  return (
    <>
      <Button
        radius={"xl"}
        onClick={() => {
          modals.open({
            withCloseButton: false,
            overlayProps: { backgroundOpacity: 0.2 },
            size: "sm",
            radius: "lg",
            children: <ModalContent initValue="compress" />,
          });
        }}
      >
        Compress & Send
      </Button>
    </>
  );
};
