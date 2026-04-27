FROM node:22-bookworm
RUN apt-get update && apt-get install -y --no-install-recommends \
    libsecret-1-0 curl python3 ca-certificates \
    && rm -rf /var/lib/apt/lists/*
ENV PATH="/root/.local/bin:${PATH}"
WORKDIR /app
COPY . .
RUN ./scripts/ci/model-dig.sh
