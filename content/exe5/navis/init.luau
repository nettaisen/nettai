--!strict
-- EXE5's navis (docs/design/content-model-v2.md §4.0), by id: the tables of the modules that define them as one, which
-- the game's init.luau returns as its `navis`. Written by tools/content/index.py (the verification workspace) from the
-- modules that are there: run it again rather than edit this, also to settle a merge conflict in it.

local merge = require("@exelib/merge")

return merge {
    -- Navis.
    require("@self/colonel"),
    require("@self/gyroman"),
    require("@self/knightman"),
    require("@self/magnetman"),
    require("@self/meddy"),
    require("@self/megaman"),
    require("@self/napalmman"),
    require("@self/numberman"),
    require("@self/protoman"),
    require("@self/searchman"),
    require("@self/shadowman"),
    require("@self/toadman"),
    require("@self/tomahawkman"),
}
