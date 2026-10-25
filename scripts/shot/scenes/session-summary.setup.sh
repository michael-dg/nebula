# The README grid (see readme-grid.setup.sh), with three of its stand-in agents also writing their
# card's SUMMARY through `nebula summary` — the sqlite one after a follow-up prompt that says nothing
# on its own — and the others writing none, so their cards show as before.
. "$HERE/scenes/readme-grid.setup.sh"
python3 - "$RUNTIME/agent" <<'PY'
import sys
path = sys.argv[1]
text = open(path).read()
hook = '"$NEBULA_SHOT_BIN" rename "$title" >/dev/null 2>&1 || true\n'
extra = '''case "$n" in
  3) "$NEBULA_SHOT_BIN" summary "Expired and refreshed tokens are covered by tests" >/dev/null 2>&1 ;;
  4) post UserPromptSubmit '{"session_id":"shot-4","prompt":"ok, go ahead"}'
     "$NEBULA_SHOT_BIN" summary "Token refreshes stop locking each other out" >/dev/null 2>&1 ;;
  5) "$NEBULA_SHOT_BIN" summary "The daemon starts in under 100ms" >/dev/null 2>&1 ;;
esac
'''
assert hook in text
open(path, "w").write(text.replace(hook, hook + extra))
PY
