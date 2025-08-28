.PHONY: all

all:
	export SSID="904_8888" && \
	export PASSWORD="wifipwd1115" && \
	export SERVER_URL="ws://192.168.112.168:8765" && \
	cargo b -r
