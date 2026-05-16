#!/bin/bash
# Quick Recipes for jetsongpio CLI
# Common GPIO patterns using shell composition

PIN=7

# ===== BASIC OPERATIONS =====
# Quick HIGH (auto-setup as OUT)
jetsongpio high $PIN

# Quick LOW (auto-setup as OUT)
jetsongpio low $PIN

# Manual setup with direction
jetsongpio setup $PIN --direction out
jetsongpio setup $PIN --direction in

# Setup with initial value
jetsongpio setup $PIN --direction out --initial high

# Set value (requires setup first)
jetsongpio setup $PIN --direction out
jetsongpio set $PIN high
jetsongpio set $PIN low

# Read value
jetsongpio read $PIN

# Cleanup
jetsongpio cleanup $PIN      # Cleanup specific pin
jetsongpio cleanup           # Cleanup all pins

# ===== COMMON PATTERNS =====

# Pulse (HIGH for 0.5s, then LOW)
jetsongpio high $PIN && sleep 0.5 && jetsongpio low $PIN

# Blink 10 times (0.1s cycle)
for i in {1..10}; do jetsongpio high $PIN; sleep 0.1; jetsongpio low $PIN; sleep 0.1; done

# Safe pattern with cleanup on exit
trap "jetsongpio low $PIN" EXIT
jetsongpio high $PIN
# Do work...
# When script ends or Ctrl+C, pin goes LOW automatically

# Check if pin is HIGH, do something
if [ "$(jetsongpio read $PIN 2>/dev/null | grep -o 'HIGH')" = "HIGH" ]; then
    echo "Pin is HIGH"
fi

# ===== ADVANCED =====

# Timeout with automatic revert (5 seconds)
jetsongpio high $PIN
timeout 5s sh -c 'while true; do sleep 1; done' 2>/dev/null || true
jetsongpio low $PIN

# Or using timeout command directly
timeout 5s bash -c 'jetsongpio high $PIN; sleep 5; jetsongpio low $PIN'

# Watch for changes
while true; do
    jetsongpio read $PIN
    sleep 0.1
done

# Toggle on multiple pins
for pin in 7 11 40; do
    jetsongpio high $pin
done
sleep 1
for pin in 7 11 40; do
    jetsongpio low $pin
done