"use client";

import { createTheme, MantineColorsTuple } from "@mantine/core";

const blue: MantineColorsTuple = [
  "#e5f4ff",
  "#cde3ff",
  "#9bc3ff",
  "#64a2ff",
  "#3986fe",
  "#1d74fe",
  "#0066ff",
  "#005be4",
  "#0051cc",
  "#0045b5",
];

export const theme = createTheme({
  focusRing: "never",
  colors: {
    // default primary
    blue,
  },
  defaultRadius: "sm",
});
