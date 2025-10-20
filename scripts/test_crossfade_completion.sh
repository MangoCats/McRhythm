#!/bin/bash
# Manual Test Script for Crossfade Completion Fix (BUG-003)
#
# This script tests the fix for the issue where passages play twice after crossfade.
# Expected behavior: Each passage plays exactly once with seamless crossfades.

set -e

echo "=== Crossfade Completion Manual Test ==="
echo "Date: $(date)"
echo ""

# Test files (4-5 minutes each)
FILE1="/home/sw/Music/Bigger,_Better,_Faster,_More/(4_Non_Blondes)Bigger,_Better,_Faster,_More-02-Superfly_.mp3"
FILE2="/home/sw/Music/Bigger,_Better,_Faster,_More/(4_Non_Blondes)Bigger,_Better,_Faster,_More-09-Dear_Mr._President.mp3"
FILE3="/home/sw/Music/Bigger,_Better,_Faster,_More/(4_Non_Blondes)Bigger,_Better,_Faster,_More-03-What's_Up_.mp3"

LOG_FILE="/home/sw/Dev/McRhythm/issues/manual_test_$(date +%Y-%m-%dT%H%M%S).log"

echo "Step 1: Starting wkmp-ap server with debug logging..."
cd /home/sw/Dev/McRhythm
RUST_LOG=debug cargo run --release -p wkmp-ap > "$LOG_FILE" 2>&1 &
SERVER_PID=$!
echo "Server PID: $SERVER_PID"
echo "Log file: $LOG_FILE"
echo ""

echo "Waiting 5 seconds for server startup..."
sleep 5

echo "Step 2: Verifying server is ready..."
if ! curl -s http://localhost:5721/api/queue > /dev/null; then
    echo "ERROR: Server not responding"
    kill $SERVER_PID
    exit 1
fi
echo "Server ready!"
echo ""

echo "Step 3: Enqueuing 3 passages..."
echo "  - Passage 1: Superfly (277s / 4.6min)"
curl -s -X POST http://localhost:5721/api/playback/enqueue \
  -H "Content-Type: application/json" \
  -d "{\"file_path\":\"$FILE1\"}" | jq .

echo "  - Passage 2: Dear Mr. President (283s / 4.7min)"
curl -s -X POST http://localhost:5721/api/playback/enqueue \
  -H "Content-Type: application/json" \
  -d "{\"file_path\":\"$FILE2\"}" | jq .

echo "  - Passage 3: What's Up (295s / 4.9min)"
curl -s -X POST http://localhost:5721/api/playback/enqueue \
  -H "Content-Type: application/json" \
  -d "{\"file_path\":\"$FILE3\"}" | jq .
echo ""

echo "Step 4: Checking queue..."
QUEUE_LEN=$(curl -s http://localhost:5721/api/queue | jq 'length')
echo "Queue length: $QUEUE_LEN passages"
echo ""

if [ "$QUEUE_LEN" -ne 3 ]; then
    echo "ERROR: Expected 3 passages in queue, got $QUEUE_LEN"
    kill $SERVER_PID
    exit 1
fi

echo "Step 5: Monitoring playback (first 60 seconds)..."
echo "Watching for:"
echo "  - PassageStarted events (should be 1 per passage)"
echo "  - [XFD-COMP-010] Crossfade completion messages"
echo "  - 'Crossfade completion handled' messages"
echo "  - NO duplicate passages"
echo ""

# Monitor for 60 seconds
for i in {1..60}; do
    sleep 1
    # Check for key events in log
    if grep -q "\[XFD-COMP-010\] Crossfade completed" "$LOG_FILE" 2>/dev/null; then
        if [ $i -ge 10 ]; then
            echo "[t+${i}s] ✓ Crossfade completion detected!"
            break
        fi
    fi
done

echo ""
echo "Step 6: Analyzing log for key events..."
echo ""

echo "=== PassageStarted Events ==="
grep "PassageStarted" "$LOG_FILE" 2>/dev/null || echo "  (none yet - passages may still be playing)"
echo ""

echo "=== Crossfade Completion Messages [XFD-COMP-010] ==="
grep "\[XFD-COMP-010\]" "$LOG_FILE" 2>/dev/null || echo "  (none yet - may not have reached crossfade point)"
echo ""

echo "=== Crossfade Completion Handling ==="
grep "Crossfade completion handled" "$LOG_FILE" 2>/dev/null || echo "  (none yet)"
echo ""

echo "=== Passage Completion Messages ==="
grep "Passage.*completed" "$LOG_FILE" 2>/dev/null | head -5 || echo "  (none yet)"
echo ""

echo "=== Queue State ==="
echo "Current queue:"
curl -s http://localhost:5721/api/queue | jq -r '.[] | "  - \(.queue_entry_id[0:8])... (order: \(.play_order))"'
echo ""

echo "=== TEST INSTRUCTIONS ==="
echo ""
echo "The server is running in background (PID: $SERVER_PID)"
echo "Log file: $LOG_FILE"
echo ""
echo "To observe the full test:"
echo "  1. Monitor log: tail -f $LOG_FILE | grep -E 'XFD-COMP|Crossfade|Passage.*completed|PassageStarted'"
echo "  2. Watch queue: watch -n 2 'curl -s http://localhost:5721/api/queue | jq length'"
echo "  3. Let playback run for ~15 minutes (2 crossfades will occur)"
echo ""
echo "SUCCESS CRITERIA:"
echo "  ✓ Each passage gets exactly ONE 'PassageStarted' event"
echo "  ✓ Two '[XFD-COMP-010] Crossfade completed' messages"
echo "  ✓ Two 'Crossfade completion handled' messages"
echo "  ✓ NO 'mixer.stop()' calls during crossfades"
echo "  ✓ Queue display updates after each crossfade"
echo "  ✓ Passage 2 does NOT play twice"
echo ""
echo "To stop the test:"
echo "  kill $SERVER_PID"
echo ""
echo "Press Ctrl+C to stop monitoring (server will keep running)"
echo "Or wait 5 more minutes to observe first crossfade..."

# Optional: Wait longer to see first crossfade
read -t 300 -p "Waiting 5 minutes for first crossfade (Ctrl+C to skip)..." || echo ""

echo ""
echo "=== FINAL LOG ANALYSIS ==="
grep -E "XFD-COMP|Crossfade|PassageStarted" "$LOG_FILE" | tail -20
