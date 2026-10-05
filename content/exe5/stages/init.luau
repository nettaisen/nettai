--!strict
-- EXE5's stages/ (docs/design/content-model-v2.md §4.0): the folder's modules that define what the game has, which
-- load when the game's init.luau requires the folder. Written by tools/content/index.py (the verification workspace)
-- from the modules that are there: run it again rather than edit this, also to settle a merge conflict in it.

-- Stages.
require("@self/netbattle")
