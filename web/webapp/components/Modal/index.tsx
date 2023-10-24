import { Stack, Box, SegmentedControl, Button } from "@mantine/core";
import { modals } from "@mantine/modals";
import { useState } from "react";
import { ShieldForm, SendForm } from "../Form";

const ModalContent = () => {
  const [value, setValue] = useState("shield");

  return (
    <Stack>
      <Box px={"md"}>
        <SegmentedControl
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
            children: <ModalContent />,
            keepMounted: true,
          });
        }}
      >
        Shield & Send
      </Button>
    </>
  );
};
