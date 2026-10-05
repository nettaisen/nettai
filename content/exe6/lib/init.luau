--!strict
-- EXE6's lib/ (docs/design/content-model-v2.md §4.0): the folder's modules that define what the game has, which load
-- when the game's init.luau requires the folder. Written by tools/content/index.py (the verification workspace) from
-- the modules that are there: run it again rather than edit this, also to settle a merge conflict in it.

-- Modules nothing else requires, loaded for what they define: the alias routines, kinds and effects the original's
-- tables number, which compat names.
require("@self/bombs/slash")
require("@self/dimming/blinding_flash")
require("@self/instant/repair")
require("@self/patch-cards/b-reflect")
