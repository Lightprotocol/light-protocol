FROM golang:1.20.3-alpine as builder

WORKDIR /app

COPY go.mod go.sum ./
RUN go mod download && go mod verify

COPY . .

ENV CGO_ENABLED=0
RUN go build -v -o /usr/local/bin/light-prover .

FROM gcr.io/distroless/base-debian11:nonroot

COPY --from=builder /usr/local/bin/light-prover /usr/local/bin/light-prover
VOLUME /config

ENTRYPOINT [ "light-prover" ]
CMD [ "start", "--config", "config.toml"]
