#!/bin/bash

# Shell script that downloads the DNS blocklist and clean it up.

# Define URL and output file
BLOCKLIST_URL="https://raw.githubusercontent.com/hagezi/dns-blocklists/main/domains/pro.txt"
OUTPUT_FILE="hagezi_clean_domains.txt"

# Download, filter, and clean the list
if curl -s "$BLOCKLIST_URL" | \
   grep -Ev '^(!|\[|\s*$)' | \
   sed -E 's/^\|\|(.+)\^$/\1/' > "$OUTPUT_FILE"; then
    echo "Success! Cleaned list saved to $OUTPUT_FILE"
    echo "Total domains: $(wc -l < "$OUTPUT_FILE")"
else
    echo "Error: Failed to download or process the blocklist." >&2
    exit 1
fi