PROJECT = "rock-paper-scissors"

all: install build test 

test: ;@echo "Testing ${PROJECT}..."; \
	yarn test;

install: ;@echo "Installing ${PROJECT}..."; \
	yarn install;

build: ;@echo "Building ${PROJECT}..."; \
	yarn build;

update: ;@echo "Updating ${PROJECT}..."; \
	git pull --rebase; \
	yarn install

clean : ;
	rm -rf node_modules


.PHONY: test install build clean update