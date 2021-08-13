#!/bin/bash

# Shamelessly stolen from https://eugene-babichenko.github.io/blog/2018/11/08/rustfmt-git-hook/

HAS_ISSUES=0

for file in $(git diff --name-only --staged); do
    FMT_RESULT="$(rustfmt --check $file 2>/dev/null || true)"
    if [ "$FMT_RESULT" != "" ]; then
        echo "$file"
        HAS_ISSUES=1
    fi
done

if [ $HAS_ISSUES -eq 0 ]; then
    exit 0
fi

echo "Your code has formatting issues in files listed above."
echo "Run \`make format\` before committing."
exit 1

