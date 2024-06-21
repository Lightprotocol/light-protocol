package logging

import (
	gnarkLogger "github.com/consensys/gnark/logger"
	"github.com/rs/zerolog"
	"os"
)

var log = zerolog.New(zerolog.ConsoleWriter{Out: os.Stderr, TimeFormat: "15:04:05"}).With().Timestamp().Logger()

func Logger() *zerolog.Logger {
	return &log
}

func SetJSONOutput() {
	log = zerolog.New(os.Stdout).With().Timestamp().Logger()
	gnarkLogger.Set(log)
}
