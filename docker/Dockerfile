FROM ubuntu:24.04

ENV DEBIAN_FRONTEND=noninteractive
WORKDIR /opt/armbian-imager

RUN apt-get update && apt-get install -y \
    ca-certificates \
    wget \
    libglib2.0-0 \
    libgtk-3-0 \
    libxcb1 \
    libx11-6 \
    libxrender1 \
    libxext6 \
    libxi6 \
    libxrandr2 \
    libxinerama1 \
    libxcursor1 \
    libxdamage1 \
    libxfixes3 \
    libdrm2 \
    libgbm1 \
    udev \
    && rm -rf /var/lib/apt/lists/*

ARG VERSION=1.2.3

RUN wget https://github.com/armbian/imager/releases/download/v${VERSION}/Armbian.Imager_${VERSION}_arm64.deb \
 && apt-get update \
 && apt-get install -y ./Armbian.Imager_${VERSION}_arm64.deb \
 && rm Armbian.Imager_${VERSION}_arm64.deb

VOLUME ["/dev", "/run/udev"]

ENTRYPOINT ["armbian-imager"]
