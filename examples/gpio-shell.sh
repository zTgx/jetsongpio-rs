#!/bin/bash
# GPIO Shell Examples
# Demonstrates using jetsongpio CLI with shell commands for various patterns
# These patterns show how to compose CLI commands instead of implementing timeout in the tool

set -e

# Configuration
PIN=7
PULSE_DURATION=0.5
BLINK_COUNT=5
BLINK_DELAY=0.2

echo "=== GPIO Shell Examples ==="
echo "Using pin: $PIN"
echo

# Example 1: Simple pulse (HIGH for a short time, then LOW)
echo "Example 1: Simple pulse (HIGH for ${PULSE_DURATION}s, then LOW)"
jetsongpio high $PIN
sleep $PULSE_DURATION
jetsongpio low $PIN
echo
sleep 1

# Example 2: Blink LED (toggle multiple times)
echo "Example 2: Blink LED ${BLINK_COUNT} times"
for i in $(seq 1 $BLINK_COUNT); do
    echo "Blink $i/$BLINK_COUNT"
    jetsongpio high $PIN
    sleep $BLINK_DELAY
    jetsongpio low $PIN
    sleep $BLINK_DELAY
done
echo
sleep 1

# Example 3: Safe pulse with trap (ensures LOW even if Ctrl+C)
echo "Example 3: Safe pulse with trap (ensures LOW even if Ctrl+C)"
cleanup() {
    echo "Cleaning up..."
    jetsongpio low $PIN
    exit 0
}
trap cleanup EXIT INT TERM

jetsongpio high $PIN
sleep $PULSE_DURATION
echo "Pulse completed"
echo
sleep 1

# Example 4: Read input pin (must be setup as IN first)
echo "Example 4: Read input pin"
jetsongpio setup $PIN --direction in
jetsongpio read $PIN
echo
sleep 1

# Example 5: Using setup with initial value
echo "Example 5: Setup with initial HIGH"
jetsongpio setup $PIN --direction out --initial high
jetsongpio read $PIN
echo
sleep 1

# Example 6: Manual setup then control
echo "Example 6: Manual setup then control"
jetsongpio setup $PIN --direction out --initial low
echo "Initial state:"
jetsongpio read $PIN
echo "Setting HIGH:"
jetsongpio set $PIN high
jetsongpio read $PIN
echo "Setting LOW:"
jetsongpio set $PIN low
jetsongpio read $PIN
echo

# Final cleanup
echo "Example 7: Cleanup specific pin"
jetsongpio cleanup $PIN
echo

echo "Example 8: Cleanup all pins"
jetsongpio setup $PIN --direction out
jetsongpio high $PIN
jetsongpio cleanup
echo "All pins cleaned up"
echo

echo "=== All examples completed ==="