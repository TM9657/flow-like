# syntax=docker/dockerfile:1.4
# ============================================================
# Stage 1: Builder
# ============================================================
FROM oven/bun:1.2-debian AS builder

WORKDIR /app

# Copy root package files for workspace resolution
COPY package.json bun.lock ./
COPY packages/ui/package.json packages/ui/
COPY apps/web/package.json apps/web/

# Install dependencies
RUN bun install --frozen-lockfile

# Copy source code
COPY packages/ui packages/ui/
COPY apps/web apps/web/

# Build arguments for environment variables
ARG NEXT_PUBLIC_API_URL=http://localhost:8080
ARG NEXT_PUBLIC_REDIRECT_URL=http://localhost:3001/callback
ARG NEXT_PUBLIC_REDIRECT_LOGOUT_URL=http://localhost:3001/

# Set environment variables for build
ENV NEXT_PUBLIC_API_URL=${NEXT_PUBLIC_API_URL}
ENV NEXT_PUBLIC_REDIRECT_URL=${NEXT_PUBLIC_REDIRECT_URL}
ENV NEXT_PUBLIC_REDIRECT_LOGOUT_URL=${NEXT_PUBLIC_REDIRECT_LOGOUT_URL}
ENV NEXT_TELEMETRY_DISABLED=1

# Build the static export
WORKDIR /app/apps/web
RUN bun run build

# ============================================================
# Stage 2: Runtime (nginx to serve static files)
# ============================================================
FROM nginx:alpine

# Copy custom nginx configuration
COPY apps/backend/docker-compose/web/nginx.conf /etc/nginx/conf.d/default.conf

# Copy static files from builder
COPY --from=builder /app/apps/web/out /usr/share/nginx/html

# Create non-root user
RUN adduser -D -u 1000 flowlike && \
    chown -R flowlike:flowlike /usr/share/nginx/html && \
    chown -R flowlike:flowlike /var/cache/nginx && \
    chown -R flowlike:flowlike /var/log/nginx && \
    touch /var/run/nginx.pid && \
    chown flowlike:flowlike /var/run/nginx.pid

USER flowlike

EXPOSE 3001

CMD ["nginx", "-g", "daemon off;"]
