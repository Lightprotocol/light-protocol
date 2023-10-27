/**
 * @jest-environment jsdom
 */
import { render, fireEvent, act } from "@testing-library/react";
import React from "react";
import { Provider } from "jotai";
import { useAction } from "./useAction";

jest.mock("./useAction"); // Mock the useAction hook

// simulate the use of useAction hook
const MockComponent = () => {
  const { shield } = useAction();

  const handleClick = async () => {
    await shield({
      token: "SOL",
      recipient: "recipientPublicKey",
      publicAmountSol: "0.001",
      appUtxo: undefined,
      confirmOptions: undefined,
      senderTokenAccount: undefined,
    });
  };

  return <button onClick={handleClick}>Shield</button>;
};

describe("useAction hook", () => {
  it("calls shield function when button is clicked", async () => {
    const shieldMock = jest.fn();
    (useAction as jest.Mock).mockReturnValue({ shield: shieldMock });

    const { getByText } = render(
      <Provider>
        <MockComponent />
      </Provider>
    );

    const button = getByText("Shield");

    await act(async () => {
      fireEvent.click(button);
    });

    expect(shieldMock).toHaveBeenCalled();
    expect(shieldMock).not.toThrow();
  });
});
