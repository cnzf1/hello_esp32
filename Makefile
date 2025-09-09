.PHONY: all debug detail clean

all:
	export SSID="904_8888" && \
	export PASSWORD="wifipwd1115" && \
	export SERVER_URL="ws://192.168.112.168:8765" && \
	cargo b -r

debug:
	export SSID="904_8888" && \
	export PASSWORD="wifipwd1115" && \
	export SERVER_URL="ws://192.168.112.168:8765" && \
	RUST_BACKTRACE=1 cargo b -r

detail:
	export SSID="904_8888" && \
	export PASSWORD="wifipwd1115" && \
	export SERVER_URL="ws://192.168.112.168:8765" && \
	RUST_BACKTRACE=full cargo b -r

clean:
	cargo clean
