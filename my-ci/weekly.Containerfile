FROM rust:bookworm
RUN apt-get update && apt-get install -y --no-install-recommends \
    curl libsecret-1-0 ca-certificates \
    && curl -fsSL https://deb.nodesource.com/setup_20.x | bash - \
    && apt-get install -y nodejs \
    && rm -rf /var/lib/apt/lists/*
RUN npm install -g @openai/codex
WORKDIR /app
COPY . .
CMD ["bash", "-lc", "codex exec --dangerously-bypass-approvals-and-sandbox \"Follow .github/.agents/skills/merge-and-release/SKILL.md and complete the full procedure.\""]
