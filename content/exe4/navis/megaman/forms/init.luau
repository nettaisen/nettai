--!strict
-- EXE4's forms (docs/design/content-model-v2.md §4.0), by id: the tables of the modules that define them as one, which
-- the game's init.luau returns as its `forms`. Written by tools/content/index.py (the verification workspace) from the
-- modules that are there: run it again rather than edit this, also to settle a merge conflict in it.

local merge = require("@exelib/merge")

return merge {
    -- Forms.
    require("@self/aquasoul"),
    require("@self/base"),
    require("@self/firesoul"),
    require("@self/gutssoul"),
    require("@self/junksoul"),
    require("@self/metalsoul"),
    require("@self/numbersoul"),
    require("@self/protosoul"),
    require("@self/rollsoul"),
    require("@self/searchsoul"),
    require("@self/thundersoul"),
    require("@self/windsoul"),
    require("@self/woodsoul"),
}
