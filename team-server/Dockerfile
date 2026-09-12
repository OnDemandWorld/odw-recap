# ODW Recap team-server — multi-stage build (distroless-adjacent, small image)
# Requires the recap-db/recap-redis services from docker-compose.suite.yml.
FROM golang:1.23-alpine AS build
WORKDIR /src
# Cache deps first for fast rebuilds
COPY go.mod go.sum* ./
RUN go mod download || true
COPY cmd ./cmd
COPY internal ./internal
RUN CGO_ENABLED=0 go build -ldflags="-s -w" -o /out/recap-team-server ./cmd/server

FROM alpine:3.20
RUN apk add --no-cache ca-certificates wget \
 && adduser -D -u 10001 recap
COPY --from=build /out/recap-team-server /usr/local/bin/recap-team-server
USER recap
EXPOSE 8080
# wget is bundled for the compose healthcheck
ENTRYPOINT ["recap-team-server"]
