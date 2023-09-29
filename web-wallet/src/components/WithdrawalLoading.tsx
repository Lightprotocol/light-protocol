import {
  Button,
  majorScale,
  Pane,
  Heading,
  Strong,
  Text,
  minorScale,
  Link,
} from "evergreen-ui";
import l1 from "../assets/svg/L1anim.svg";
import l2 from "../assets/svg/L2anim.svg";
import l3 from "../assets/svg/L3anim.svg";
import l4 from "../assets/svg/L4.svg";
import l1err from "../assets/svg/L1error.svg";
import l2err from "../assets/svg/L2error.svg";
import l3err from "../assets/svg/L3error.svg";
import { WithdrawalFeeTable } from "./WithdrawalFeeTable";
import {
  NavigationStatus,
  cancelActionAtom,
  processStatusAtom,
  processingErrorAtom,
  setIdleAtom,
} from "../state/navigationAtoms";
import { useAtom } from "jotai";
import {
  formInputAmountAtom,
  formInputRecipientAtom,
} from "../state/utilAtoms";

export const WithdrawalLoading = () => {
  const [recipient] = useAtom(formInputRecipientAtom);
  const [amount] = useAtom(formInputAmountAtom);
  const [_, setIdle] = useAtom(setIdleAtom);
  const [__, cancelAction] = useAtom(cancelActionAtom);
  const [unshieldStatus] = useAtom(processStatusAtom);
  const [processingError] = useAtom(processingErrorAtom);
  const goBack = (e: any) => {
    e.preventDefault();
    cancelAction();
  };

  return (
    <>
      <Pane>
        <Pane
          display="flex"
          textAlign="center"
          marginTop={majorScale(4)}
        ></Pane>
      </Pane>

      <>
        <Pane display="flex">
          <Heading size={300}>Unshielding Overview</Heading>
        </Pane>
        <Pane marginBottom={majorScale(3)}>
          {/* @ts-ignore */}
          <WithdrawalFeeTable amount={amount} recipient={recipient} />
        </Pane>
        <Pane marginBottom={majorScale(0)}>
          {unshieldStatus === NavigationStatus.SIGNING ? (
            <StageSign error={processingError} />
          ) : unshieldStatus === NavigationStatus.PREPARING ? (
            <StagePrepare error={processingError} />
          ) : unshieldStatus === NavigationStatus.PROCESSING ? (
            <StageShield error={processingError} />
          ) : unshieldStatus === NavigationStatus.DONE ? (
            <StageDone recipient={recipient} />
          ) : (
            ""
          )}
        </Pane>
      </>

      <Pane marginTop={majorScale(2)}>
        {unshieldStatus === NavigationStatus.PROCESSING && !processingError && (
          <Pane
            marginBottom={majorScale(3)}
            marginTop={majorScale(3)}
            display="flex"
            flexDirection="column"
            alignItems="start"
          >
            <Pane display="flex">
              <Text textAlign="start" size={300}>
                If you have any questions,
              </Text>
              <Link
                size={300}
                marginLeft="4px"
                color="neutral"
                target="_blank"
                href="https://discord.gg/WDAAaX6je2"
              >
                we're here to help.
              </Link>
            </Pane>
            <Pane display="flex" marginTop="8px">
              <Text size={300} textAlign="start">
                You may close this page at any time. Your shielding is almost
                complete. If there are any issues, we'll notify you here.
              </Text>
            </Pane>
          </Pane>
        )}

        {processingError && (
          <>
            <Pane
              marginBottom={majorScale(3)}
              marginTop={majorScale(1)}
              display="flex"
              flexDirection="column"
              alignItems="start"
            >
              <Pane display="flex" marginTop="0px">
                {unshieldStatus === NavigationStatus.SIGNING ? (
                  <Text size={300} textAlign="start">
                    This error means that either <br />- You rejected the
                    signature request using your wallet, or <br />- signature
                    verification failed. Please go back and retry.
                  </Text>
                ) : (
                  <Text size={300} textAlign="start">
                    Whoops. This is on us. An error usually means that either{" "}
                    <br />- Solana is experiencing temporary network congestion,
                    or <br />- something else went wrong. Please go back and
                    retry.
                  </Text>
                )}
              </Pane>
              <Pane display="flex" marginTop="8px">
                <Text textAlign="start" size={300}>
                  If you have any questions,
                </Text>
                <Link
                  size={300}
                  marginLeft="4px"
                  color="neutral"
                  target="_blank"
                  href="https://discord.gg/WDAAaX6je2"
                >
                  we're here to help.
                </Link>
              </Pane>
            </Pane>
            <Button
              onClick={(e: any) => goBack(e)}
              width="-webkit-fill-available"
              size="large"
            >
              Go back
            </Button>
          </>
        )}

        {!processingError &&
          (unshieldStatus === NavigationStatus.PROCESSING ||
            unshieldStatus === NavigationStatus.DONE) && (
            <Button
              appearance="default"
              onClick={(e: any) => {
                e.preventDefault();
                // setDone();
                setIdle();
              }}
              size="large"
              width="-webkit-fill-available"
            >
              Go home
            </Button>
          )}
      </Pane>
    </>
  );
};

const RenderProgress = ({ header = "", description = "", height = 21 }) => {
  return (
    <Pane
      height={minorScale(height)}
      display="flex"
      justifyContent="start"
      alignItems="start"
    >
      <Pane
        display="flex"
        flexDirection="column"
        alignItems="start"
        textAlign="initial"
      >
        <Text>
          <Strong>{header} </Strong>
        </Text>
        <Pane display="flex" marginTop="0px">
          <Text size={300}>{description}</Text>
        </Pane>
      </Pane>
    </Pane>
  );
};

const StageSign = ({ error = false }) => {
  return (
    <>
      <Pane display="flex">
        <Pane>
          <object
            type="image/svg+xml"
            data={error ? l1err : l1}
            width={"34px"}
            // @ts-ignore
            alt=""
          />
        </Pane>
        <Pane
          display="flex"
          flexDirection="column"
          alignItems="start"
          marginLeft={majorScale(2)}
        >
          <RenderProgress
            header={
              !error ? "Signing transaction" : "Signing transaction failed"
            }
            description={
              !error &&
              "Please review the transaction and sign with your wallet."
            }
          />
          <RenderProgress header={"Preparing unshield"} description={""} />
          <Pane>
            <RenderProgress
              header={"Unshielding your crypto"}
              description={""}
              height={10}
            ></RenderProgress>
          </Pane>
        </Pane>
      </Pane>
    </>
  );
};

const StagePrepare = ({ error = false }) => {
  return (
    <>
      <Pane display="flex">
        <Pane>
          <object
            type="image/svg+xml"
            data={error ? l2err : l2}
            width={"34px"}
            // @ts-ignore
            alt=""
          />
        </Pane>
        <Pane
          display="flex"
          flexDirection="column"
          alignItems="start"
          marginLeft={majorScale(2)}
        >
          <RenderProgress header={"Transaction signed"} description={""} />
          <RenderProgress
            header={!error ? "Preparing unshield" : "Preparing unshield failed"}
            description={
              !error &&
              "We're preparing the transaction. This takes between \n5-30 seconds. Do not leave this page."
            }
          />
          <Pane>
            <RenderProgress
              header={"Unshielding your crypto"}
              description={""}
              height={10}
            ></RenderProgress>
          </Pane>
        </Pane>
      </Pane>
    </>
  );
};

const StageShield = ({ error = false }) => {
  return (
    <>
      <Pane display="flex">
        <Pane>
          <object
            type="image/svg+xml"
            data={!error ? l3 : l3err}
            width={"34px"}
            // @ts-ignore
            alt=""
          />
        </Pane>
        <Pane
          display="flex"
          flexDirection="column"
          alignItems="start"
          marginLeft={majorScale(2)}
        >
          <RenderProgress header={"Transaction signed"} description={""} />
          <RenderProgress header={"Unshield prepared"} description={""} />
          <Pane>
            <RenderProgress
              header={!error ? "Unshielding your crypto" : "Unshield failed"}
              description={
                !error &&
                "Solana is verifying the unshielding transaction. This takes a few minutes. Thanks for your patience."
              }
              height={18}
            ></RenderProgress>
          </Pane>
        </Pane>
      </Pane>
    </>
  );
};

const StageDone = ({ recipient = null }) => {
  let explorerLink = `https://explorer.solana.com/address/${recipient}`;
  return (
    <>
      <Pane display="flex">
        <Pane>
          {/* @ts-ignore */}
          <object type="image/svg+xml" data={l4} width={"34px"} alt="" />
        </Pane>
        <Pane
          display="flex"
          flexDirection="column"
          alignItems="start"
          marginLeft={majorScale(2)}
        >
          <RenderProgress header={"Transaction signed"} description={""} />
          <RenderProgress header={"Unshield prepared"} description={""} />
          <Pane>
            <RenderProgress
              header={"Unshield completed"}
              // @ts-ignore
              description={
                recipient ? (
                  <Pane
                    borderRadius="10px"
                    paddingLeft={"12px"}
                    paddingRight={"12px"}
                    paddingTop={"4px"}
                    paddingBottom={"4px"}
                    elevation={0}
                    marginTop={majorScale(1)}
                    backgroundColor={"#f3f5f6"}
                  >
                    <Link
                      size={300}
                      href={explorerLink}
                      target="_blank"
                      color="neutral"
                    >
                      View recipient
                    </Link>
                  </Pane>
                ) : (
                  ""
                )
              }
              height={14}
            ></RenderProgress>
          </Pane>
        </Pane>
      </Pane>
    </>
  );
};
