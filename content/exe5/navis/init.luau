--!strict
-- EXE5's navis/ (docs/design/content-model-v2.md §4.0): the folder's modules that define what the game has, which load
-- when the game's init.luau requires the folder. Written by tools/content/index.py (the verification workspace) from
-- the modules that are there: run it again rather than edit this, also to settle a merge conflict in it.

-- Navis (a navi chip's navi is defined with its chips).
require("@self/colonel")
require("@self/gyroman")
require("@self/knightman")
require("@self/meddy")
require("@self/megaman")
require("@self/napalmman")
require("@self/protoman")
require("@self/searchman")
require("@self/shadowman")
require("@self/tomahawkman")
-- Chips: a module defining one, or a series module defining several.
require("@self/colonel/chip")
require("@self/gyroman/chip")
require("@self/knightman/chip")
require("@self/meddy/chip")
require("@self/napalmman/chip")
require("@self/protoman/chip")
require("@self/searchman/chip")
require("@self/shadowman/chip")
require("@self/tomahawkman/chip")
-- Forms.
require("@self/megaman/forms/base")
require("@self/megaman/forms/colonelsoul")
require("@self/megaman/forms/gyrosoul")
require("@self/megaman/forms/knightsoul")
require("@self/megaman/forms/magnetsoul")
require("@self/megaman/forms/meddysoul")
require("@self/megaman/forms/napalmsoul")
require("@self/megaman/forms/numbersoul")
require("@self/megaman/forms/protosoul")
require("@self/megaman/forms/searchsoul")
require("@self/megaman/forms/shadowsoul")
require("@self/megaman/forms/toadsoul")
require("@self/megaman/forms/tomahawksoul")
