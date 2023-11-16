import { useState } from "react";
import { UnstyledButton, Menu, Image, Group } from "@mantine/core";
import { IconChevronDown } from "@tabler/icons-react";
import classes from "../../styles/TokenPicker.module.css";

const TOKENS = [
  {
    label: "SOL",
    image:
      "https://jup.ag/_next/image?url=https%3A%2F%2Fraw.githubusercontent.com%2Fsolana-labs%2Ftoken-list%2Fmain%2Fassets%2Fmainnet%2FSo11111111111111111111111111111111111111112%2Flogo.png&w=64&q=75",
  },
  {
    label: "USDC",
    image:
      "https://jup.ag/_next/image?url=https%3A%2F%2Fraw.githubusercontent.com%2Fsolana-labs%2Ftoken-list%2Fmain%2Fassets%2Fmainnet%2FEPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v%2Flogo.png&w=64&q=75",
  },
];

export function TokenPicker({ form }: { form: any }) {
  const [opened, setOpened] = useState(false);
  const [selected, setSelected] = useState(TOKENS[0]);
  const items = TOKENS.map((item) => (
    <Menu.Item
      data-testid={`token-option-${item.label}`}
      leftSection={
        <Image src={item.image} alt="" width={18} height={18} radius={"xl"} />
      }
      onClick={() => {
        setSelected(item);
        form.setFieldValue("token", item.label);
      }}
      key={item.label}
    >
      {item.label}
    </Menu.Item>
  ));

  return (
    <Menu
      onOpen={() => setOpened(true)}
      onClose={() => setOpened(false)}
      radius="md"
      width="target"
      withinPortal
    >
      <Menu.Target>
        <UnstyledButton
          className={classes.control}
          data-testid="token-dropdown"
          data-expanded={opened || undefined}
        >
          <Group gap="xs">
            <Image
              alt=""
              src={selected.image}
              width={22}
              height={22}
              radius={"xl"}
            />
            <span className={classes.label}>{selected.label}</span>
          </Group>
          <IconChevronDown size="1rem" className={classes.icon} stroke={1.5} />
        </UnstyledButton>
      </Menu.Target>
      <Menu.Dropdown>{items}</Menu.Dropdown>
    </Menu>
  );
}
