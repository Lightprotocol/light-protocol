# Use the official Node.js 14 image as the base image
FROM --platform=linux/amd64 node:16 as builder

# Set the working directory inside the container
WORKDIR /usr/src/app

# Copy the original pnpm-workspace.yaml file
COPY ./pnpm-workspace.yaml ./

# Create a new pnpm-workspace.yaml file with the excluded directories removed
RUN sed '/cli/d; /psp-examples/d' pnpm-workspace.yaml > pnpm-workspace-new.yaml

# Start the second stage of the build
FROM --platform=linux/amd64 node:16

# Install PNPM globally inside the container
RUN npm install -g pnpm

# Set the working directory inside the container
WORKDIR /usr/src/app

# Copy package.json and pnpm-lock.yaml for all projects
COPY ./package.json ./nx.json ./ 
#./pnpm-lock.yaml
COPY ./relayer/package.json ./relayer/
COPY ./zk.js/package.json ./zk.js/
COPY ./prover.js/package.json ./prover.js/
COPY ./circuit-lib/circuit-lib.js/package.json ./circuit-lib/circuit-lib.js/
COPY ./circuit-lib/circuit-lib.circom/package.json ./circuit-lib/circuit-lib.circom/
COPY ./system-programs/package.json ./system-programs/
# Copy the tsconfig
COPY ./tsconfig ./tsconfig/

# Copy the new pnpm-workspace.yaml file from the builder stage
COPY --from=builder /usr/src/app/pnpm-workspace-new.yaml ./pnpm-workspace.yaml

# Install dependencies using PNPM
RUN pnpm install 

# Copy the rest of the projects
COPY ./relayer ./relayer/
COPY ./zk.js ./zk.js/
COPY ./prover.js ./prover.js/
COPY ./circuit-lib ./circuit-lib/
COPY ./system-programs ./system-programs/
# light-anchor is required by zkjs<-system-programs

COPY light-anchor /usr/local/bin/
RUN chmod +x /usr/local/bin/light-anchor
# Copy the build script and make it executable
COPY ./scripts/build.sh ./scripts/build.sh
RUN chmod +x ./scripts/build.sh

# Run the build script
RUN ./scripts/build.sh

# Copy the docker-entrypoint.sh file
COPY ./docker-entrypoint.sh /usr/local/bin/

# Set the entrypoint
ENTRYPOINT ["/usr/local/bin/docker-entrypoint.sh"]

# Set the command to run your application
CMD ["node", "relayer/lib/index.js"]