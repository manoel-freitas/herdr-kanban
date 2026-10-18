#!/bin/sh
# Adds the default Kanban keybinding (prefix+i) to the herdr config on install.
# Idempotent; never fails the install (a missing key is not worth aborting for).
cfg="${HERDR_CONFIG_PATH:-$HOME/.config/herdr/config.toml}"

if [ ! -f "$cfg" ]; then
  echo "kanban: $cfg not found; skipping keybinding. See README to add it by hand."
  exit 0
fi
if grep -q 'manoel-freitas.kanban.open' "$cfg"; then
  echo "kanban: keybinding already present in $cfg"
  exit 0
fi

if printf '\n[[keys.command]]\nkey = "prefix+i"\ntype = "plugin_action"\ncommand = "manoel-freitas.kanban.open"\ndescription = "kanban"\n' >> "$cfg"; then
  echo "kanban: added prefix+i to $cfg (run: herdr server reload-config)"
else
  echo "kanban: could not write $cfg; see README to add the keybinding by hand."
fi
exit 0
