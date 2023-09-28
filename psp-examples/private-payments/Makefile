PROJECT = "private-payments"

all: install test 

test: ;@echo "Testing ${PROJECT}..."; \
	yarn test-local;

install: ;@echo "Installing ${PROJECT}..."; \
	yarn install;

update: ;@echo "Updating ${PROJECT}..."; \
	git pull --rebase; \
	yarn install

clean : ;
	rm -rf node_modules


.PHONY: test install clean update