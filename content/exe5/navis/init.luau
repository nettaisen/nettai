--!strict
-- EXE5's navis (docs/design/content-model-v2.md §4.0), by id: the tables of the modules that define them as one, which
-- the game's init.luau returns as its `navis`. Written by tools/content/index.py (the verification workspace) from the
-- modules that are there: run it again rather than edit this, also to settle a merge conflict in it.

local merge = require("@exelib/merge")

return merge {
    -- Navis.
    ["navis/colonel"] = require("@self/colonel"),
    ["navis/gyroman"] = require("@self/gyroman"),
    ["navis/knightman"] = require("@self/knightman"),
    ["navis/magnetman"] = require("@self/magnetman"),
    ["navis/meddy"] = require("@self/meddy"),
    ["navis/megaman"] = require("@self/megaman"),
    ["navis/napalmman"] = require("@self/napalmman"),
    ["navis/numberman"] = require("@self/numberman"),
    ["navis/protoman"] = require("@self/protoman"),
    ["navis/searchman"] = require("@self/searchman"),
    ["navis/shadowman"] = require("@self/shadowman"),
    ["navis/toadman"] = require("@self/toadman"),
    ["navis/tomahawkman"] = require("@self/tomahawkman"),
}
