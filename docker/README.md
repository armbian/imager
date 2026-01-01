# Armbian Imager – Docker (ARM64)

This directory provides a Docker-based method for running Armbian Imager
on ARM64 systems where host GLIBC versions are too old to run the prebuilt binary.

This is intended as a **community-supported workaround** and does not replace
native packages.

## Use cases
- Older Armbian / Debian / Ubuntu releases
- GLIBC version mismatch errors
- Clean, reproducible runtime environment

## Requirements
- ARM64 host
- Docker
- X11 display server
- Privileged container access (for block device flashing)

## Build
```bash
docker build -t armbian-imager-docker docker/
