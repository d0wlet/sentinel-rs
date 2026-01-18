#!/bin/bash
# High-speed log generator
rm -f test.log
touch test.log

echo "Starting high-speed log generation to test.log..."
COUNTER=0
while true; do
    COUNTER=$((COUNTER+1))
    
    # Normal log
    echo "[INFO] Transaction $COUNTER processed successfully at $(date)" >> test.log
    
    # Every ~100 logs, inject an error
    if (( COUNTER % 100 == 0 )); then
        echo "[ERROR] Database connection failed for user_id=$COUNTER" >> test.log
    fi
    
    # Every ~500 logs, inject a panic
    if (( COUNTER % 500 == 0 )); then
        echo "panic!: Kernel panic at main.rs:42" >> test.log
    fi

    # Every ~700 logs, inject a JSON Error (Modern Style)
    if (( COUNTER % 700 == 0 )); then
        echo "{\"level\": \"error\", \"msg\": \"Critical JSON failure in API\", \"service\": \"payment\"}" >> test.log
    fi

    # Don't sleep too much, we want speed.
    # On linux 'yes' is faster but let's emulate some app behavior.
    # Removing sleep to test maximum ingestion rate.
done
