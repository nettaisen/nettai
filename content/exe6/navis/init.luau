--!strict
-- EXE6's navis (docs/design/content-model-v2.md §4.0), by id: the tables of the modules that define them as one, which
-- the game's init.luau returns as its `navis`. Written by tools/content/index.py (the verification workspace) from the
-- modules that are there: run it again rather than edit this, also to settle a merge conflict in it.

local merge = require("@exelib/merge")

return merge {
    -- Navis.
    require("@self/chargeman"),
    require("@self/dustman"),
    require("@self/elecman"),
    require("@self/eraseman"),
    require("@self/groundman"),
    require("@self/heatman"),
    require("@self/megaman"),
    require("@self/protoman"),
    require("@self/slashman"),
    require("@self/spoutman"),
    require("@self/tenguman"),
    require("@self/tomahawkman"),
}
