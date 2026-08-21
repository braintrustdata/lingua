#!/usr/bin/env bash
# Verify the regenerated TypeScript bindings differ only in JSDoc comments.
set -u
strip() {
  grep -v '^ \*' | grep -v '^/\*\*' | tr -d ' \n' | md5sum
}
status=0
for f in $(git diff --name-only bindings/typescript/src/generated/openai/); do
  old=$(git show "HEAD:$f" | strip)
  new=$(strip < "$f")
  if [ "$old" = "$new" ]; then
    echo "SAME  $f"
  else
    echo "DIFF  $f"
    status=1
  fi
done
exit "$status"
