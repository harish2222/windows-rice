#!/bin/bash
count=$(winget list --source winget 2>/dev/null | tail -n +4 | grep -c '.')
echo "$count (winget)"
