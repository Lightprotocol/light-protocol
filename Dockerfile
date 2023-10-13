# Use the official Node.js 16 image as the base image
FROM node:16 as builder

# Set the working directory inside the container
WORKDIR /usr/src/app

# Copy the built relayer and node_modules to the working directory
COPY ./relayer /usr/src/app/relayer


# Copy the docker-entrypoint.sh file
COPY ./docker-entrypoint.sh /usr/local/bin/

# Set the entrypoint
ENTRYPOINT ["/usr/local/bin/docker-entrypoint.sh"]

# Set the command to run your application
CMD ["node", "relayer/lib/index.js"]
