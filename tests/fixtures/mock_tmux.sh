#!/bin/bash
#
# Mock tmux script for testing LogPilot without requiring actual tmux
# This simulates tmux commands for integration testing
#

set -e

SESSION_NAME=""
LOG_FILE=""
PID_FILE=""

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        new-session)
            shift
            while [[ $# -gt 0 ]]; do
                case $1 in
                    -d) shift ;; # detached mode
                    -s) SESSION_NAME="$2"; shift 2 ;;
                    -n) shift 2 ;; # window name, ignore
                    *) break ;;
                esac
            done
            ;;
        send-keys)
            shift
            # Skip target and command for now
            shift 2
            ;;
        pipe-pane)
            shift
            # Simulate pipe-pane -t <target> "cat >> <fifo>"
            # Just set up a background process that writes to the fifo
            ;;
        list-sessions)
            if [[ -n "$SESSION_NAME" ]]; then
                echo "$SESSION_NAME: 1 windows"
            fi
            ;;
        list-panes)
            echo "${SESSION_NAME}:0.0: [80x24] [history 0/10000, 0 lines] {active}"
            ;;
        *)
            echo "Unknown command: $1" >&2
            exit 1
            ;;
    esac
    shift
done

# Create session directory
if [[ -n "$SESSION_NAME" ]]; then
    SESSION_DIR="/tmp/mock-tmux-$SESSION_NAME-$$"
    mkdir -p "$SESSION_DIR"
    
    # Create log file
    LOG_FILE="$SESSION_DIR/output.log"
    touch "$LOG_FILE"
    
    # Start a background "log producer" process
    # This simulates kubectl logs -f or similar
    (
        counter=0
        while true; do
            echo "[$(date -Iseconds)] INFO mock-service: Log line $counter"
            ((counter++))
            sleep 0.1
        done
    ) >> "$LOG_FILE" &
    
    echo $! > "$SESSION_DIR/producer.pid"
    echo "$SESSION_DIR" > "/tmp/mock-tmux-session-$SESSION_NAME"
    
    echo "Mock session created: $SESSION_NAME"
    echo "Log file: $LOG_FILE"
fi

# Cleanup function
cleanup() {
    if [[ -n "$SESSION_NAME" ]]; then
        SESSION_DIR="/tmp/mock-tmux-$SESSION_NAME-$$"
        if [[ -f "$SESSION_DIR/producer.pid" ]]; then
            kill $(cat "$SESSION_DIR/producer.pid") 2>/dev/null || true
        fi
        rm -rf "$SESSION_DIR"
        rm -f "/tmp/mock-tmux-session-$SESSION_NAME"
    fi
}

trap cleanup EXIT
