--!strict
-- EXE6's top module (docs/design/content-model-v2.md §4.0): a load of the game runs it, and what it requires, in turn,
-- is what loads. It requires the game's rules and its folders; each folder's init.luau requires the folder's modules
-- that define what the game has, and a definition none of them lists is refused. The order is the load order, which no
-- key or handle depends on. Written by tools/content/index.py (the verification workspace) from the modules that are
-- there: run it again rather than edit this, also to settle a merge conflict in it.

require("@self/rules")
require("@self/chips")
require("@self/navis")
require("@self/stages")
require("@self/cards")
require("@self/navicust")
require("@self/lib")
