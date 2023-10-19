"use client";
import React, { useCallback, useState } from "react";
import {
  Box,
  Image,
  ModalOverlay,
  Switch,
  useDisclosure,
} from "@chakra-ui/react";
import { sleep } from "@lightprotocol/zk.js";
import { useTransactions } from "@/state/hooks/useTransactions";

import { FormControl, FormLabel, Input, Select } from "@chakra-ui/react";

import {
  Modal,
  ModalContent,
  ModalHeader,
  ModalCloseButton,
  ModalBody,
} from "@chakra-ui/react";

const Button = ({
  primary,
  children,
  onClick,
  type,
}: {
  primary: boolean;
  children: React.ReactNode;
  onClick: any;
  type?: "button" | "submit" | "reset" | undefined;
}) => (
  <button
    onClick={onClick}
    type={type}
    style={{
      backgroundColor: primary ? "#06f" : "#d3d3d3",
      color: "#fff",
      borderRadius: "40px",
      padding: "10px 20px",
      border: "none",
      marginRight: "10px",
      cursor: "pointer",
    }}
  >
    {children}
  </button>
);

const SendForm = () => {
  return (
    <form>
      <FormControl id="amount">
        <FormLabel>Amount</FormLabel>
        <Input type="number" />
        <Select placeholder="Select token">
          <option value="SOL">SOL</option>
          <option value="USDC">USDC</option>
        </Select>
      </FormControl>
      <FormControl id="recipient">
        <FormLabel>Recipient</FormLabel>
        <Input type="text" />
      </FormControl>
      <Button primary type="submit" onClick={() => {}}>
        Preview Send
      </Button>
    </form>
  );
};

// TODO: this should render - the recipient - a connect walle tbutton if required with state there.
// also add all the form validation stuff
// and displays based on validation and state.
const ShieldForm = () => {
  return (
    <form>
      <FormControl id="shield">
        <FormLabel>Shield Amount</FormLabel>
        <Input type="number" />
        <Select placeholder="Select token">
          <option value="SOL">SOL</option>
          <option value="USDC">USDC</option>
        </Select>
      </FormControl>
      <Button primary type="submit" onClick={() => {}}>
        Preview Shield
      </Button>
    </form>
  );
};

const FormWithSwitch = ({
  formType,
  setFormType,
  children,
}: {
  formType: any;
  setFormType: any;
  children: React.ReactNode;
}) => {
  return (
    <>
      <Switch
        color="blue"
        onChange={() => setFormType(formType === "send" ? "shield" : "send")}
      />
      {children}
    </>
  );
};

const Dialog = ({
  isOpen,
  onClose,
  header,
  children,
}: {
  isOpen: any;
  onClose: any;
  header: string;
  children: any;
}) => {
  return (
    <Modal isOpen={isOpen} onClose={onClose}>
      <ModalOverlay bg="rgba(0, 0, 0, 0.1)" />
      <ModalContent>
        <ModalHeader>{header}</ModalHeader>
        <ModalCloseButton />
        <ModalBody>{children}</ModalBody>
      </ModalContent>
    </Modal>
  );
};

export const Navbar = () => {
  const { transactions, sync, syncError, isSyncing } = useTransactions();
  const { isOpen, onOpen, onClose } = useDisclosure();
  const [formType, setFormType] = useState("send");

  const openDialog = useCallback(async () => {
    await sleep(40);
    onOpen();
  }, [onOpen]);

  return (
    <Box
      p="1em"
      w="full"
      display="flex"
      justifyContent="space-between"
      alignItems="center"
      borderBottom="1px solid rgba(0, 0, 0, .066)"
    >
      <Image src="placeholderimg" alt="Logo" boxSize="40px" />
      <Box display="flex" justifyContent="flex-end" alignItems="center">
        <Button
          primary
          onClick={() => {
            setFormType("shield");
            openDialog();
          }}
        >
          Shield & Send
        </Button>

        <Dialog
          isOpen={isOpen}
          onClose={onClose}
          header={
            formType === "send" ? "Send Tokens Privately" : "Shield Tokens"
          }
        >
          <FormWithSwitch formType={formType} setFormType={setFormType}>
            {formType === "send" ? <SendForm /> : <ShieldForm />}
          </FormWithSwitch>
        </Dialog>
      </Box>
    </Box>
  );
};
