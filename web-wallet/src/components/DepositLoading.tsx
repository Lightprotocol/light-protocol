//@ts-check
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
import { DepositFeeTable } from "./DepositFeeTable";
import { useAtom } from "jotai";
import {
  NavigationStatus,
  cancelActionAtom,
  processStatusAtom,
  processingErrorAtom,
} from "../state/navigationAtoms";
import { baseUnitAmountAtom } from "../state/utilAtoms";

export const DepositLoading = () => {
  const [processStatus] = useAtom(processStatusAtom);
  const [isProcessError] = useAtom(processingErrorAtom);
  const [_, cancelAction] = useAtom(cancelActionAtom);
  const [baseUnitAmount] = useAtom(baseUnitAmountAtom);
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
          <Heading size={300}>Shielding Overview</Heading>
        </Pane>
        <Pane marginBottom={majorScale(3)}>
          {/* @ts-ignore */}
          <DepositFeeTable baseUnitAmount={baseUnitAmount} />
        </Pane>
        <Pane marginBottom={majorScale(0)}>
          {processStatus === NavigationStatus.SIGNING ? (
            <StageSign error={isProcessError} />
          ) : processStatus === NavigationStatus.PREPARING ? (
            <StagePrepare error={isProcessError} />
          ) : processStatus === NavigationStatus.PROCESSING ? (
            <StageShield error={isProcessError} />
          ) : processStatus === NavigationStatus.DONE ? (
            <StageDone />
          ) : (
            ""
          )}
        </Pane>
      </>

      <Pane marginTop={majorScale(2)}>
        {processStatus === NavigationStatus.PROCESSING && !isProcessError && (
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
                Do not close this tab. Else you risk loss of funds. Background
                processing will be back soon. If there are any issues, we'll
                notify you here.
              </Text>
            </Pane>
          </Pane>
        )}

        {isProcessError && (
          <>
            <Pane
              marginBottom={majorScale(3)}
              marginTop={majorScale(1)}
              display="flex"
              flexDirection="column"
              alignItems="start"
            >
              <Pane display="flex" marginTop="0px">
                {processStatus === NavigationStatus.SIGNING ? (
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

        {!isProcessError && processStatus === NavigationStatus.DONE && (
          <Button
            appearance="default"
            onClick={(e: any) => {
              e.preventDefault();
              cancelAction();
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
          <RenderProgress header={"Preparing shield"} description={""} />
          <Pane>
            <RenderProgress
              header={"Shielding your crypto"}
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
            header={!error ? "Preparing shield" : "Preparing shield failed"}
            description={
              !error &&
              "We're preparing the transaction. This takes between 5-20 seconds. Do not close this tab. Else you risk loss of funds."
            }
          />
          <Pane>
            <RenderProgress
              header={"Shielding your crypto"}
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
          <RenderProgress header={"Shield prepared"} description={""} />
          <Pane>
            <RenderProgress
              header={!error ? "Shielding your crypto" : "Shield failed"}
              description={
                !error &&
                "Solana is verifying the shielding transaction. This can take a few minutes. Do not close this tab. Else you risk loss of funds. Thanks for your patience."
              }
              height={18}
            ></RenderProgress>
          </Pane>
        </Pane>
      </Pane>
    </>
  );
};

const StageDone = () => {
  return (
    <>
      <Pane display="flex">
        <Pane>
          {" "}
          {/*@ts-ignore */}
          <object type="image/svg+xml" data={l4} width={"34px"} alt="" />
        </Pane>
        <Pane
          display="flex"
          flexDirection="column"
          alignItems="start"
          marginLeft={majorScale(2)}
        >
          <RenderProgress header={"Transaction signed"} description={""} />
          <RenderProgress header={"Shield prepared"} description={""} />
          <Pane>
            <RenderProgress
              header={"Shield completed"}
              description={"Your shielded balance has been updated."}
              height={14}
            ></RenderProgress>
          </Pane>
        </Pane>
      </Pane>
    </>
  );
};
