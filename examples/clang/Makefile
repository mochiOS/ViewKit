VIEWKIT_ROOT := ../..
VIEWKIT_INCLUDE := $(VIEWKIT_ROOT)/lib/include
VIEWKIT_LIBRARY_DIR := $(VIEWKIT_ROOT)/target/release
VIEWKIT_LIBRARY := $(VIEWKIT_LIBRARY_DIR)/libviewkit.so

BUILD_DIR := build
TARGET := $(BUILD_DIR)/mochios-builder
SOURCE := main.c

CC ?= clang

CPPFLAGS := \
	-D_POSIX_C_SOURCE=200809L \
	-I$(VIEWKIT_INCLUDE)

CFLAGS := \
	-std=c17 \
	-O2 \
	-g \
	-Wall \
	-Wextra \
	-Wpedantic

LDFLAGS := \
	-L$(VIEWKIT_LIBRARY_DIR) \
	-Wl,-rpath,'$$ORIGIN/../../../target/release'

LDLIBS := \
	-lviewkit

.PHONY: all
all: $(TARGET)

.PHONY: generate
generate:
	cd $(VIEWKIT_ROOT) && ./scripts/generate.sh

.PHONY: viewkit
viewkit: generate
	cargo build \
		--release \
		--manifest-path $(VIEWKIT_ROOT)/Cargo.toml

$(TARGET): $(SOURCE) viewkit
	mkdir -p $(BUILD_DIR)
	$(CC) \
		$(CPPFLAGS) \
		$(CFLAGS) \
		$(SOURCE) \
		$(LDFLAGS) \
		$(LDLIBS) \
		-o $(TARGET)

.PHONY: run
run: $(TARGET)
	./$(TARGET) ./.config

.PHONY: clean
clean:
	rm -rf $(BUILD_DIR)

.PHONY: clean-all
clean-all: clean
	cargo clean \
		--manifest-path $(VIEWKIT_ROOT)/Cargo.toml