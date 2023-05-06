#!/bin/bash

# Set the static binary name
SCRIPT_DIR="$(dirname "$BASH_SOURCE")"
binary_name="/Users/adam/dev/shellbot/target/release/shellbot"
# Check if stdin is empty
if [ -t 0 ]; then
  # If stdin is empty, create a temporary file and open it in the editor
  tmpfile=$(mktemp)
  trap "rm -f $tmpfile" EXIT
  ${EDITOR} "$tmpfile"

  # Use edited file as input source
  input_source="$tmpfile"
else
  # Otherwise, use stdin as input source
  input_source="/dev/stdin"
fi

console_width=$(tput cols)
# Invoke the binary with the input source and pipe the output through fold
"$binary_name" < "$input_source" | fold -s -w "$console_width"
