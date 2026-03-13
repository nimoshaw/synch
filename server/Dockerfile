FROM golang:1.21-alpine AS builder

WORKDIR /app

# Install git if needed for go mod
RUN apk add --no-cache git

# Copy go mod and sum files
COPY go.mod ./
# If go.sum exists, copy it. For now, we will run go mod tidy
RUN go mod tidy

# Copy the source code
COPY . .

# Build the application
RUN CGO_ENABLED=0 GOOS=linux go build -o synch-relay ./cmd/relay

# Final stage
FROM alpine:3.19

WORKDIR /root/

COPY --from=builder /app/synch-relay .

EXPOSE 8080

CMD ["./synch-relay"]
