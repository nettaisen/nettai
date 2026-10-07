--!strict
-- EXE4's chips (docs/design/content-model-v2.md §4.0), by id: the tables of the modules that define them as one, which
-- the game's init.luau returns as its `chips`. Written by tools/content/index.py (the verification workspace) from the
-- modules that are there: run it again rather than edit this, also to settle a merge conflict in it.

local merge = require("@exelib/merge")

return merge {
    -- Chips: a module defining one, or a series module defining several.
    require("@self/cannon"),
    require("@self/darkbomb"),
    require("@self/drkcanon"),
    require("@self/drklance"),
    require("@self/drkrecov"),
    require("@self/drkspred"),
    require("@self/drkstage"),
    require("@self/drksword"),
    require("@self/drkvulcn"),
    require("@self/lance"),
    require("@self/recov"),
}
