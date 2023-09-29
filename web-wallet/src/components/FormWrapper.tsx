import React from "react";
import {
  Button,
  Pane,
  ArrowDownIcon,
  ArrowUpIcon,
  majorScale,
  defaultTheme,
  ThemeProvider,
  mergeTheme,
} from "evergreen-ui";
import {
  Action,
  navigationAtom,
  updateActionAtom,
} from "../state/navigationAtoms";
import { useAtom } from "jotai";

export const FormWrapper = ({ children }) => {
  const [navigationState] = useAtom(navigationAtom);
  const [_, updateAction] = useAtom(updateActionAtom);

  const theme = mergeTheme(defaultTheme, {
    components: {
      Button: {
        baseStyle: {
          fontSize: "14px",
          backgroundColor: "white",
          borderBottomRightRadius: "0px",
          borderBottomLeftRadius: "0px",
          borderTopLeftRadius: "15px",
          borderTopRightRadius: "15px",

          borderColor: "white",

          _active: {
            color: "#0066FF",
            borderBottomColor: "#0066FF",
          },
          _hover: {
            color: "#0066FF",
          },
          _focus: {
            color: "#0066FF",
          },
          _keyDown: {
            color: "#0066FF",
          },
          boxShadow: "none",
        },
      },
    },
  });
  return (
    <>
      <Pane
        display="flex"
        flexDirection="column"
        justifyContent="center"
      ></Pane>
      <Pane
        display="flex"
        justifyContent="center"
        alignItems="center"
        marginBottom="16px"
      >
        <ThemeProvider value={theme}>
          <Button
            marginRight="2px"
            appearance=""
            isActive={navigationState.action == Action.SHIELD ? true : false}
            size="large"
            height="3.5em"
            fontWeight="400"
            fontSize="17px"
            onClick={(e: any) => {
              e.preventDefault();
              updateAction(Action.SHIELD);
            }}
            width="-webkit-fill-available"
          >
            <ArrowUpIcon size={13} marginRight={majorScale(1)} />
            Shield
          </Button>
          <Button
            appearance=""
            isActive={navigationState.action == Action.UNSHIELD ? true : false}
            size="large"
            height="3.5em"
            fontWeight="400"
            fontSize="17px"
            onClick={(e: any) => {
              e.preventDefault();
              updateAction(Action.UNSHIELD);
            }}
            width="-webkit-fill-available"
          >
            <ArrowDownIcon size={13} marginRight={majorScale(1)} />
            Unshield
          </Button>
        </ThemeProvider>
      </Pane>

      {children}
    </>
  );
};
