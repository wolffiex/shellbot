#!/bin/bash

# Set the static binary name
script_path="$(realpath "$0")"
script_dir="$(dirname "$script_path")"
binary_name="$script_dir/target/release/shellbot"

console_width=$(tput cols)
# Check if stdin is empty
if [ -t 0 ]; then
  # If stdin is empty, create a temporary file and open it in the editor
  tmpfile=$(mktemp)
  trap "rm -f $tmpfile" EXIT
  ${EDITOR} "$tmpfile"
  echo "🧑 $USER"
  cat "$tmpfile" | fold -s -w "$console_width"
  # Use edited file as input source
  input_source="$tmpfile"
else
  # Otherwise, use stdin as input source
  input_source="/dev/stdin"
fi


separator="───  ⋅ ∙ ∘ ༓ ∘ ⋅ ⋅  ───"
padding_width=$(( (console_width - ${#separator}) / 2 ))
padded_separator=$(printf "%*s%s" ${padding_width} '' "${separator}")
echo "${padded_separator}"
echo -e "🤖 shellbot"
# Invoke the binary with the input source and pipe the output through fold
"$binary_name" < "$input_source" | fold -s -w "$console_width"
