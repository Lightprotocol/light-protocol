# List of subdirectories containing project Makefiles
PROJECT_DIRS := private-payments streaming-payments encrypted-messaging rock-paper-scissors swap

# Targets to pass to the sub-Makefiles
TARGETS := all

.PHONY: $(PROJECT_DIRS)

all: $(PROJECT_DIRS)

$(PROJECT_DIRS):
	$(MAKE) -C $@ $(TARGETS)

.PHONY: clean

clean:
	for dir in $(PROJECT_DIRS); do \
    	$(MAKE) -C $$dir clean; \
	done
