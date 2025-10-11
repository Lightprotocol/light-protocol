FROM golang:1.25-alpine AS builder

WORKDIR /app

COPY go.mod go.sum ./
RUN go mod download && go mod verify

COPY . .

ENV CGO_ENABLED=0
RUN go build -v -o /usr/local/bin/light-prover .

FROM gcr.io/distroless/base-debian11:nonroot

COPY --from=builder /usr/local/bin/light-prover /usr/local/bin/light-prover

WORKDIR /proving-keys

COPY --chown=nonroot:nonroot ./proving-keys/v1_* /proving-keys/
COPY --chown=nonroot:nonroot ./proving-keys/v2_* /proving-keys/

WORKDIR /

ENTRYPOINT [ "light-prover" ]
CMD [ "start", "--preload-keys", "rpc", "--keys-dir", "/proving-keys/" ]
