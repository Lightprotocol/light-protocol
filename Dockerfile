FROM node:16 as builder

WORKDIR /usr/src/app

COPY ./relayer /usr/src/app/relayer

COPY ./docker-entrypoint.sh /usr/local/bin/

ENTRYPOINT ["/usr/local/bin/docker-entrypoint.sh"]

CMD ["node", "relayer/lib/index.js"]
