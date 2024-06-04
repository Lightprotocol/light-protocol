#!/bin/sh

set -e

until docker compose exec db pg_isready -h localhost -p 5432; do
  >&2 echo "Postgres is unavailable - sleeping"
  sleep 1
done

>&2 echo "Postgres is up"