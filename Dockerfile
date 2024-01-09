FROM node:16 as builder

WORKDIR /usr/src/app

COPY ./rpc /usr/src/app/rpc

COPY ./docker-entrypoint.sh /usr/local/bin/

ENTRYPOINT ["/usr/local/bin/docker-entrypoint.sh"]

CMD ["node", "rpc/lib/index.js"]
