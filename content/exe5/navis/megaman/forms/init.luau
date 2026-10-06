--!strict
-- EXE5's forms (docs/design/content-model-v2.md §4.0), by id: the tables of the modules that define them as one, which
-- the game's init.luau returns as its `forms`. Written by tools/content/index.py (the verification workspace) from the
-- modules that are there: run it again rather than edit this, also to settle a merge conflict in it.

local merge = require("@exelib/merge")

return merge {
    -- Forms.
    require("@self/base"),
    require("@self/colonelsoul"),
    require("@self/gyrosoul"),
    require("@self/knightsoul"),
    require("@self/magnetsoul"),
    require("@self/meddysoul"),
    require("@self/napalmsoul"),
    require("@self/numbersoul"),
    require("@self/protosoul"),
    require("@self/searchsoul"),
    require("@self/shadowsoul"),
    require("@self/toadsoul"),
    require("@self/tomahawksoul"),
}
