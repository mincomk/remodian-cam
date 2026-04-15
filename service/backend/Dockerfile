FROM rust:bookworm AS builder
WORKDIR /app

# System libraries required by eframe (transitive dep via remodian-control)
RUN apt-get update && apt-get install -y --no-install-recommends \
    libxcb-render0-dev \
    libxcb-shape0-dev \
    libxcb-xfixes0-dev \
    libxkbcommon-dev \
    libssl-dev \
    libgl1-mesa-dev \
    pkg-config \
    && rm -rf /var/lib/apt/lists/*

# Copy workspace manifests
COPY Cargo.toml Cargo.lock ./

# Remove firmware member — it requires the ESP toolchain and can't be built here
RUN sed -i '/firmware\/remodian-firmware/d' Cargo.toml

# Copy only the crates needed for remodian-backend
COPY core/ core/
COPY dev/vision-server/ dev/vision-server/
COPY dev/calibration-ui/ dev/calibration-ui/
COPY service/backend/ service/backend/

RUN cargo build --release --package remodian-backend

FROM debian:bookworm-slim
WORKDIR /app

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    libssl3 \
    libgl1 \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/remodian-backend /app/

EXPOSE 3000
CMD ["/app/remodian-backend"]
