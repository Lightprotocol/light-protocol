import { Pane, Text } from "evergreen-ui";

export default function SetAmountButton(
  {
    setAmount,
    amount,
    label = "",
    descriptor = "",
  }: {
    setAmount: Function;
    amount: number;
    label?: string;
    descriptor?: string;
  },
  ...props: any
) {
  return (
    <>
      <Pane justifyContent="flex-end" display="flex" alignItems="baseline">
        {label && (
          <Text cursor="pointer" size={300} onClick={() => setAmount(amount)}>
            {label}
          </Text>
        )}
        <Text
          cursor="pointer"
          textDecoration="underline"
          size={300}
          color="muted"
          onMouseOver={(e: any) => (e.target.style.color = "blue")}
          onMouseLeave={(e: any) => (e.target.style.color = "#696f8c")}
          marginLeft="2px"
          onClick={() => {
            if (amount > 0) {
              setAmount(amount);
            }
          }}
          {...props}
        >
          {amount} {descriptor}
        </Text>
      </Pane>
    </>
  );
}
