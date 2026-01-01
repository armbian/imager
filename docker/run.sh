#!/usr/bin/env bash
set -e

xhost +local:docker >/dev/null

docker run --rm -it \
  --privileged \
  --network host \
  -e DISPLAY=$DISPLAY \
  -v /tmp/.X11-unix:/tmp/.X11-unix \
  -v /dev:/dev \
  -v /run/udev:/run/udev \
  armbian-imager-docker

xhost -local:docker >/dev/null
