prog :=xnixperms

debug ?=

$(info debug is $(debug))

ifdef debug
  release :=
  target :=debug
  extension :=debug
else
  release :=--release
  target :=release
  extension :=
endif

build:
	cargo build $(release) --target x86_64-unknown-linux-musl
	#strip -s target/$(target)/kenshi
	strip --strip-unneeded target/x86_64-unknown-linux-musl/$(target)/kenshi
	cp -fv target/x86_64-unknown-linux-musl/$(target)/kenshi .
	upx -9 kenshi

debug:
	cargo build $(release)
	cp -fv target/$(target)/kenshi .



test:
	cargo test -- --nocapture

install:
	cp target/$(target)/$(prog) ~/bin/$(prog)-$(extension)

all: build install
 
help:
	@echo "usage: make $(prog) [debug=1]"
