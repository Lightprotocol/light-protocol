import { Stack, Box, SegmentedControl, Button } from "@mantine/core";
import { modals } from "@mantine/modals";
import { useState, SetStateAction } from "react";
import { ShieldForm, SendForm } from "../Form";

export const ModalContent = ({
  initValue = "shield",
}: {
  initValue?: string; // TODO: enforce strict type checking
}) => {
  const [value, setValue] = useState(initValue);
  return (
    <Stack data-testid="shield-send-modal">
      <Box px={"md"}>
        <SegmentedControl
          data-testid="shield-send-control"
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
            children: <ModalContent initValue="shield" />,
          });
        }}
      >
        Shield & Send
      </Button>
    </>
  );
};
