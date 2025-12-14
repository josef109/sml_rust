# Deployment Guide

## Embedded Linux (systemd)

1. Build release binary
2. Copy binary to `/usr/local/bin`
3. Install `sml.service`
4. Enable and start service

```sh
sudo systemctl enable sml.service
sudo systemctl start sml.service
```

## Docker Deployment

```sh
cross build -v --release --target armv7-unknown-linux-gnueabihf
```