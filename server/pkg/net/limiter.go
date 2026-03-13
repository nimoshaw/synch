package net

import (
	"sync"

	"golang.org/x/time/rate"
)

// RateLimiter manages rate limiting for multiple keys (e.g., IPs or NodeIDs)
type RateLimiter struct {
	limiters sync.Map
	rate     rate.Limit
	burst    int
}

// NewRateLimiter creates a new RateLimiter with specified rate and burst
func NewRateLimiter(r rate.Limit, b int) *RateLimiter {
	return &RateLimiter{
		rate:  r,
		burst: b,
	}
}

// Allow checks if a key is allowed to proceed
func (rl *RateLimiter) Allow(key string) bool {
	l, _ := rl.limiters.LoadOrStore(key, rate.NewLimiter(rl.rate, rl.burst))
	return l.(*rate.Limiter).Allow()
}

// LimiterManager holds different limiters for different stages
type LimiterManager struct {
	ConnLimiter    *RateLimiter // Limits new connections by IP
	MessageLimiter *RateLimiter // Limits messages per authenticated client
}

// DefaultRelayLimiters returns a standard configuration for the relay
func DefaultRelayLimiters() *LimiterManager {
	return &LimiterManager{
		// 5 connections per second, burst of 20 - generous for multiple rooms/tabs
		ConnLimiter: NewRateLimiter(5, 20),
		// 100 messages per second, burst of 200 - high enough for real-time sync but protects against spam
		MessageLimiter: NewRateLimiter(100, 200),
	}
}
