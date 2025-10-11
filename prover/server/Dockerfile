FROM golang:1.20.3-alpine AS builder

WORKDIR /app

COPY go.mod go.sum ./
RUN go mod download && go mod verify

COPY . .

ENV CGO_ENABLED=0
RUN go build -v -o /usr/local/bin/light-prover .

FROM gcr.io/distroless/base-debian11:nonroot

COPY --from=builder /usr/local/bin/light-prover /usr/local/bin/light-prover

WORKDIR /proving-keys

COPY --chown=nonroot:nonroot ./proving-keys/mainnet* /proving-keys/
COPY --chown=nonroot:nonroot ./proving-keys/inclusion* /proving-keys/ 
COPY --chown=nonroot:nonroot ./proving-keys/non-inclusion_* /proving-keys/
COPY --chown=nonroot:nonroot ./proving-keys/combined_* /proving-keys/

WORKDIR /

ENTRYPOINT [ "light-prover" ]
CMD [ "start", "--run-mode", "rpc", "--keys-dir", "/proving-keys/" ]
