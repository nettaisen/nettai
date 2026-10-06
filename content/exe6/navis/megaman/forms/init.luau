--!strict
-- EXE6's forms (docs/design/content-model-v2.md §4.0), by id: the tables of the modules that define them as one, which
-- the game's init.luau returns as its `forms`. Written by tools/content/index.py (the verification workspace) from the
-- modules that are there: run it again rather than edit this, also to settle a merge conflict in it.

local merge = require("@exelib/merge")

return merge {
    -- Forms.
    require("@self/base"),
    require("@self/chargecross"),
    require("@self/chargecross_beast"),
    require("@self/dustcross"),
    require("@self/dustcross_beast"),
    require("@self/eleccross"),
    require("@self/eleccross_beast"),
    require("@self/erasecross"),
    require("@self/erasecross_beast"),
    require("@self/falzar_beast"),
    require("@self/falzar_beast_over"),
    require("@self/gregar_beast"),
    require("@self/gregar_beast_over"),
    require("@self/groundcross"),
    require("@self/groundcross_beast"),
    require("@self/heatcross"),
    require("@self/heatcross_beast"),
    require("@self/slashcross"),
    require("@self/slashcross_beast"),
    require("@self/spoutcross"),
    require("@self/spoutcross_beast"),
    require("@self/tengucross"),
    require("@self/tengucross_beast"),
    require("@self/tomahawkcross"),
    require("@self/tomahawkcross_beast"),
}
