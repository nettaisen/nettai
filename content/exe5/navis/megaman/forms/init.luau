--!strict
-- EXE5's forms (docs/design/content-model-v2.md §4.0), by id: the tables of the modules that define them as one, which
-- the game's init.luau returns as its `forms`. Written by tools/content/index.py (the verification workspace) from the
-- modules that are there: run it again rather than edit this, also to settle a merge conflict in it.

local merge = require("@exelib/merge")

return merge {
    -- Forms.
    ["navis/megaman/forms/base"] = require("@self/base"),
    ["navis/megaman/forms/colonelsoul"] = require("@self/colonelsoul"),
    ["navis/megaman/forms/gyrosoul"] = require("@self/gyrosoul"),
    ["navis/megaman/forms/knightsoul"] = require("@self/knightsoul"),
    ["navis/megaman/forms/magnetsoul"] = require("@self/magnetsoul"),
    ["navis/megaman/forms/meddysoul"] = require("@self/meddysoul"),
    ["navis/megaman/forms/napalmsoul"] = require("@self/napalmsoul"),
    ["navis/megaman/forms/numbersoul"] = require("@self/numbersoul"),
    ["navis/megaman/forms/protosoul"] = require("@self/protosoul"),
    ["navis/megaman/forms/searchsoul"] = require("@self/searchsoul"),
    ["navis/megaman/forms/shadowsoul"] = require("@self/shadowsoul"),
    ["navis/megaman/forms/toadsoul"] = require("@self/toadsoul"),
    ["navis/megaman/forms/tomahawksoul"] = require("@self/tomahawksoul"),
}
