--!strict
-- EXE6's forms (docs/design/content-model-v2.md §4.0), by id: the tables of the modules that define them as one, which
-- the game's init.luau returns as its `forms`. Written by tools/content/index.py (the verification workspace) from the
-- modules that are there: run it again rather than edit this, also to settle a merge conflict in it.

local merge = require("@exelib/merge")

return merge {
    -- Forms.
    ["navis/megaman/forms/base"] = require("@self/base"),
    ["navis/megaman/forms/chargecross"] = require("@self/chargecross"),
    ["navis/megaman/forms/chargecross_beast"] = require("@self/chargecross_beast"),
    ["navis/megaman/forms/dustcross"] = require("@self/dustcross"),
    ["navis/megaman/forms/dustcross_beast"] = require("@self/dustcross_beast"),
    ["navis/megaman/forms/eleccross"] = require("@self/eleccross"),
    ["navis/megaman/forms/eleccross_beast"] = require("@self/eleccross_beast"),
    ["navis/megaman/forms/erasecross"] = require("@self/erasecross"),
    ["navis/megaman/forms/erasecross_beast"] = require("@self/erasecross_beast"),
    ["navis/megaman/forms/falzar_beast"] = require("@self/falzar_beast"),
    ["navis/megaman/forms/falzar_beast_over"] = require("@self/falzar_beast_over"),
    ["navis/megaman/forms/gregar_beast"] = require("@self/gregar_beast"),
    ["navis/megaman/forms/gregar_beast_over"] = require("@self/gregar_beast_over"),
    ["navis/megaman/forms/groundcross"] = require("@self/groundcross"),
    ["navis/megaman/forms/groundcross_beast"] = require("@self/groundcross_beast"),
    ["navis/megaman/forms/heatcross"] = require("@self/heatcross"),
    ["navis/megaman/forms/heatcross_beast"] = require("@self/heatcross_beast"),
    ["navis/megaman/forms/slashcross"] = require("@self/slashcross"),
    ["navis/megaman/forms/slashcross_beast"] = require("@self/slashcross_beast"),
    ["navis/megaman/forms/spoutcross"] = require("@self/spoutcross"),
    ["navis/megaman/forms/spoutcross_beast"] = require("@self/spoutcross_beast"),
    ["navis/megaman/forms/tengucross"] = require("@self/tengucross"),
    ["navis/megaman/forms/tengucross_beast"] = require("@self/tengucross_beast"),
    ["navis/megaman/forms/tomahawkcross"] = require("@self/tomahawkcross"),
    ["navis/megaman/forms/tomahawkcross_beast"] = require("@self/tomahawkcross_beast"),
}
