"use client";

import { Button, createTheme, MantineColorsTuple } from "@mantine/core";
import classes from "./Button.module.css";

const blue: MantineColorsTuple = [
  "#e5f4ff",
  "#F2F6FF",
  "#9bc3ff",
  "#64a2ff",
  "#3986fe",
  "#1d74fe",
  "#0066ff",
  "#005be4",
  "#0051cc",
  "#0045b5",
];

const grey: MantineColorsTuple = [
  "#FFFFFF", // 0 white
  "#FAFAFA",
  "#F5F5F5",
  "#EBEBEB", // 3 anti flash white
  "#DCDEE5", // 4 platinum
  "#9DA3AE", // 5 cadet grey
  "#8D8D8D",
  "#393940",
  "#1F1D1C",
  "#141311", // black
];

export const theme = createTheme({
  focusRing: "never",
  colors: {
    // default primary
    blue,
    grey,
  },
  defaultRadius: "sm",
  components: {
    Button: Button.extend({
      classNames: classes,
    }),
  },
});
