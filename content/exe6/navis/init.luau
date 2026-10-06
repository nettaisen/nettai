--!strict
-- EXE6's navis (docs/design/content-model-v2.md §4.0), by id: the tables of the modules that define them as one, which
-- the game's init.luau returns as its `navis`. Written by tools/content/index.py (the verification workspace) from the
-- modules that are there: run it again rather than edit this, also to settle a merge conflict in it.

local merge = require("@exelib/merge")

return merge {
    -- Navis.
    ["navis/chargeman"] = require("@self/chargeman"),
    ["navis/dustman"] = require("@self/dustman"),
    ["navis/elecman"] = require("@self/elecman"),
    ["navis/eraseman"] = require("@self/eraseman"),
    ["navis/groundman"] = require("@self/groundman"),
    ["navis/heatman"] = require("@self/heatman"),
    ["navis/megaman"] = require("@self/megaman"),
    ["navis/protoman"] = require("@self/protoman"),
    ["navis/slashman"] = require("@self/slashman"),
    ["navis/spoutman"] = require("@self/spoutman"),
    ["navis/tenguman"] = require("@self/tenguman"),
    ["navis/tomahawkman"] = require("@self/tomahawkman"),
}
