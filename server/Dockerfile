FROM golang:1.25-alpine AS builder

WORKDIR /app

# Install git if needed for go mod
RUN apk add --no-cache git

# Copy go mod and sum files first for layer caching
COPY go.mod go.sum ./
RUN go mod download

# Copy the source code
COPY . .

# Build the application
RUN CGO_ENABLED=0 GOOS=linux go build -ldflags "-s -w" -o synch-relay ./cmd/relay

# Final stage
FROM alpine:3.19

WORKDIR /app

COPY --from=builder /app/synch-relay .
COPY healthcheck.sh .
RUN chmod +x healthcheck.sh

EXPOSE 8080

CMD ["./synch-relay"]

