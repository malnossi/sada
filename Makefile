# Sada Makefile
# High-performance cross-platform audio streaming desktop app in Tauri + Rust + Svelte

SHELL := /bin/bash

# Color helpers for premium terminal output
COLOR_RESET   = \033[0m
COLOR_SUCCESS = \033[32m
COLOR_INFO    = \033[36m
COLOR_WARNING = \033[33m
COLOR_ERROR   = \033[31m

# Automatic OS Detection
ifeq ($(OS),Windows_NT)
    DETECTED_OS := Windows
else
    UNAME_S := $(shell uname -s)
    ifeq ($(UNAME_S),Linux)
        DETECTED_OS := Linux
    endif
    ifeq ($(UNAME_S),Darwin)
        DETECTED_OS := macOS
    endif
endif

# Force audio encoders to link statically so the final bundle has no system dependencies
export LAME_STATIC := 1


.DEFAULT_GOAL := help

.PHONY: all
all: install build ## Install dependencies and build the application in release mode for the current host OS

.PHONY: install
install: ## Install frontend npm dependencies
	@echo -e "$(COLOR_INFO)Installing frontend node dependencies...$(COLOR_RESET)"
	npm install
	@echo -e "$(COLOR_SUCCESS)Frontend dependencies installed successfully.$(COLOR_RESET)"
	@echo -e "$(COLOR_WARNING)Ensure system libraries are installed. Run 'make install-deps' to setup your system dependencies automatically.$(COLOR_RESET)"

.PHONY: install-deps
install-deps: ## Auto-detect host OS and install native audio encoding dependencies
ifeq ($(DETECTED_OS),macOS)
	$(MAKE) install-deps-mac
else ifeq ($(DETECTED_OS),Linux)
	$(MAKE) install-deps-linux
else ifeq ($(DETECTED_OS),Windows)
	$(MAKE) install-deps-windows
else
	@echo -e "$(COLOR_ERROR)Unsupported operating system for automated dependency installation.$(COLOR_RESET)"
	@echo -e "Please refer to the README.md for manual system setup."
endif

.PHONY: install-deps-mac
install-deps-mac: ## Install native audio dependencies on macOS via Homebrew
	@echo -e "$(COLOR_INFO)Installing native dependencies for macOS via Homebrew...$(COLOR_RESET)"
	brew update && brew install lame opus libvorbis fdk-aac pkg-config
	@echo -e "$(COLOR_SUCCESS)macOS native dependencies installed successfully!$(COLOR_RESET)"

.PHONY: install-deps-linux
install-deps-linux: ## Install native audio & Tauri dependencies on Linux (Debian/Ubuntu) via apt-get
	@echo -e "$(COLOR_INFO)Installing native dependencies for Linux via apt-get...$(COLOR_RESET)"
	sudo apt-get update && sudo apt-get install -y \
		build-essential \
		pkg-config \
		libasound2-dev \
		libmp3lame-dev \
		libopus-dev \
		libvorbis-dev \
		libfdk-aac-dev \
		libgtk-3-dev \
		webkit2gtk-4.1 \
		libssl-dev \
		libayatana-appindicator3-dev \
		librsvg2-dev
	@echo -e "$(COLOR_SUCCESS)Linux native dependencies installed successfully!$(COLOR_RESET)"

.PHONY: install-deps-windows
install-deps-windows: ## Display instructions for setting up native audio dependencies on Windows via vcpkg
	@echo -e "$(COLOR_INFO)================ Windows Native Dependencies Setup ================(COLOR_RESET)"
	@echo -e "To compile Sada on Windows, follow these vcpkg installation guidelines:"
	@echo -e "1. Clone Microsoft vcpkg: 'git clone https://github.com/microsoft/vcpkg.git'"
	@echo -e "2. Run bootstrap script inside the cloned vcpkg directory: '.\\bootstrap-vcpkg.bat'"
	@echo -e "3. Install packages: '.\\vcpkg.exe install mp3lame:x64-windows opus:x64-windows libvorbis:x64-windows fdk-aac:x64-windows'"
	@echo -e "4. Set VCPKG_ROOT environment variable to the path of your cloned vcpkg folder."
	@echo -e "5. Enable dynamic linking: '[System.Environment]::SetEnvironmentVariable(\"VCPKGRS_DYNAMIC\", \"1\", \"User\")'"
	@echo -e "$(COLOR_INFO)====================================================================(COLOR_RESET)"

.PHONY: build
build: ## Auto-detect host OS and build the native production installer
	@echo -e "$(COLOR_INFO)Auto-detected Host Operating System: $(DETECTED_OS)$(COLOR_RESET)"
ifeq ($(DETECTED_OS),macOS)
	$(MAKE) build-mac
else ifeq ($(DETECTED_OS),Linux)
	$(MAKE) build-linux
else ifeq ($(DETECTED_OS),Windows)
	$(MAKE) build-windows
else
	@echo -e "$(COLOR_ERROR)Could not auto-detect operating system. Run specific build target manually (e.g. 'make build-mac').$(COLOR_RESET)"
endif

.PHONY: build-mac
build-mac: ## Build the macOS release bundle (dmg / app)
ifeq ($(DETECTED_OS),macOS)
	@echo -e "$(COLOR_INFO)Building Sada macOS release bundle...$(COLOR_RESET)"
	npm run tauri build
	@echo -e "$(COLOR_SUCCESS)macOS build completed successfully! Installer found in src-tauri/target/release/bundle/dmg/$(COLOR_RESET)"
else
	@echo -e "$(COLOR_ERROR)Error: macOS build target can only be executed on a macOS host system due to Apple hardware SDK requirements.$(COLOR_RESET)"
	@exit 1
endif

.PHONY: build-linux
build-linux: ## Build the Linux release bundle (deb / AppImage)
ifeq ($(DETECTED_OS),Linux)
	@echo -e "$(COLOR_INFO)Building Sada Linux release bundle...$(COLOR_RESET)"
	npm run tauri build
	@echo -e "$(COLOR_SUCCESS)Linux build completed successfully! Installer found in src-tauri/target/release/bundle/deb/$(COLOR_RESET)"
else
	@echo -e "$(COLOR_ERROR)Error: Linux build target can only be executed on a Linux host system due to WebKitGTK build environments.$(COLOR_RESET)"
	@exit 1
endif

.PHONY: build-windows
build-windows: ## Build the Windows release bundle (msi / exe)
ifeq ($(DETECTED_OS),Windows)
	@echo -e "$(COLOR_INFO)Building Sada Windows release bundle...$(COLOR_RESET)"
	npm run tauri build
	@echo -e "$(COLOR_SUCCESS)Windows build completed successfully! Installer found in src-tauri\\target\\release\\bundle\\msi\\$(COLOR_RESET)"
else
	@echo -e "$(COLOR_WARNING)Warning: Building natively on a non-Windows machine is not fully supported by standard MSVC compilers.$(COLOR_RESET)"
	@echo -e "If you have 'cargo-xwin' and Windows SDKs configured, attempting build..."
	npm run tauri build -- --target x86_64-pc-windows-msvc
endif

.PHONY: dev
dev: ## Start the hot-reloading development environment (frontend + backend)
	@echo -e "$(COLOR_INFO)Starting Sada tauri dev server...$(COLOR_RESET)"
	npm run tauri dev

.PHONY: check
check: ## Run compiler and type checks (Rust cargo check + Svelte svelte-check)
	@echo -e "$(COLOR_INFO)Running Rust backend compiler check...$(COLOR_RESET)"
	cd src-tauri && cargo check
	@echo -e "$(COLOR_INFO)Running Svelte frontend type checks...$(COLOR_RESET)"
	npx svelte-check
	@echo -e "$(COLOR_SUCCESS)All checks passed successfully!$(COLOR_RESET)"

.PHONY: clean
clean: ## Clean build files, artifacts, and target folders
	@echo -e "$(COLOR_INFO)Cleaning cargo target and frontend build outputs...$(COLOR_RESET)"
	rm -rf dist
	cd src-tauri && cargo clean
	@echo -e "$(COLOR_SUCCESS)Clean completed.$(COLOR_RESET)"

.PHONY: help
help: ## Display this help information
	@echo -e "$(COLOR_INFO)Sada Makefile commands:$(COLOR_RESET)"
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.*?## "}; {printf "  $(COLOR_SUCCESS)%-22s$(COLOR_RESET) %s\n", $$1, $$2}'
