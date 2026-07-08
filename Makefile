.PHONY: all build install install-user uninstall install-daemon uninstall-daemon

PREFIX ?= /usr/local
BINDIR ?= $(PREFIX)/bin
LOCAL_BIN ?= $(HOME)/.local/bin
SYSTEMD_USER_DIR ?= $(HOME)/.config/systemd/user

all: build

build:
	export PATH="$(HOME)/.cargo/bin:$$PATH" && cargo build --release

install: build
	install -Dm755 target/release/rune "$(DESTDIR)$(BINDIR)/rune"
	@echo "Rune installed system-wide to $(DESTDIR)$(BINDIR)/rune"

install-user: build
	mkdir -p "$(LOCAL_BIN)"
	install -m755 target/release/rune "$(LOCAL_BIN)/rune"
	@echo "Rune installed locally to $(LOCAL_BIN)/rune"
	@echo "Ensure $(LOCAL_BIN) is added to your PATH environment variable."

install-daemon:
	mkdir -p "$(SYSTEMD_USER_DIR)"
	@echo "[Unit]" > "$(SYSTEMD_USER_DIR)/rune-daemon.service"
	@echo "Description=Rune Clipboard Daemon Collector" >> "$(SYSTEMD_USER_DIR)/rune-daemon.service"
	@echo "After=default.target" >> "$(SYSTEMD_USER_DIR)/rune-daemon.service"
	@echo "" >> "$(SYSTEMD_USER_DIR)/rune-daemon.service"
	@echo "[Service]" >> "$(SYSTEMD_USER_DIR)/rune-daemon.service"
	@echo "ExecStart=$(shell which rune 2>/dev/null || echo $(LOCAL_BIN)/rune) daemon" >> "$(SYSTEMD_USER_DIR)/rune-daemon.service"
	@echo "Restart=on-failure" >> "$(SYSTEMD_USER_DIR)/rune-daemon.service"
	@echo "" >> "$(SYSTEMD_USER_DIR)/rune-daemon.service"
	@echo "[Install]" >> "$(SYSTEMD_USER_DIR)/rune-daemon.service"
	@echo "WantedBy=default.target" >> "$(SYSTEMD_USER_DIR)/rune-daemon.service"
	systemctl --user daemon-reload
	systemctl --user enable rune-daemon.service
	systemctl --user start rune-daemon.service
	@echo "Systemd user service 'rune-daemon.service' created, enabled, and started."

uninstall-daemon:
	systemctl --user stop rune-daemon.service || true
	systemctl --user disable rune-daemon.service || true
	rm -f "$(SYSTEMD_USER_DIR)/rune-daemon.service"
	systemctl --user daemon-reload
	@echo "Rune daemon systemd user service removed."

uninstall: uninstall-daemon
	rm -f "$(DESTDIR)$(BINDIR)/rune"
	rm -f "$(LOCAL_BIN)/rune"
	@echo "Rune binaries uninstalled successfully."
