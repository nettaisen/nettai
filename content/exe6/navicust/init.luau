--!strict
-- EXE6's navicust programs (docs/design/content-model-v2.md §4.0), by id: the tables of the modules that define them
-- as one, which the game's init.luau returns as its `navicust_programs`. Written by tools/content/index.py (the
-- verification workspace) from the modules that are there: run it again rather than edit this, also to settle a merge
-- conflict in it.

local merge = require("@exelib/merge")

return merge {
    -- NaviCust programs.
    ["navicust/airshoes"] = require("@self/airshoes"),
    ["navicust/antidmg"] = require("@self/antidmg"),
    ["navicust/attack-1"] = require("@self/attack-1"),
    ["navicust/attckmax"] = require("@self/attckmax"),
    ["navicust/autoheal"] = require("@self/autoheal"),
    ["navicust/battery"] = require("@self/battery"),
    ["navicust/beat"] = require("@self/beat"),
    ["navicust/bodypack"] = require("@self/bodypack"),
    ["navicust/bugstop"] = require("@self/bugstop"),
    ["navicust/bustpack"] = require("@self/bustpack"),
    ["navicust/charge-1"] = require("@self/charge-1"),
    ["navicust/chargmax"] = require("@self/chargmax"),
    ["navicust/chpshufl"] = require("@self/chpshufl"),
    ["navicust/collect"] = require("@self/collect"),
    ["navicust/custom1"] = require("@self/custom1"),
    ["navicust/custom2"] = require("@self/custom2"),
    ["navicust/fish"] = require("@self/fish"),
    ["navicust/fldrpak1"] = require("@self/fldrpak1"),
    ["navicust/fldrpak2"] = require("@self/fldrpak2"),
    ["navicust/flotshoe"] = require("@self/flotshoe"),
    ["navicust/fstbarr"] = require("@self/fstbarr"),
    ["navicust/gigfldr1"] = require("@self/gigfldr1"),
    ["navicust/hp-100"] = require("@self/hp-100"),
    ["navicust/hp-200"] = require("@self/hp-200"),
    ["navicust/hp-300"] = require("@self/hp-300"),
    ["navicust/hp-400"] = require("@self/hp-400"),
    ["navicust/hp-50"] = require("@self/hp-50"),
    ["navicust/hp-500"] = require("@self/hp-500"),
    ["navicust/humor"] = require("@self/humor"),
    ["navicust/jungle"] = require("@self/jungle"),
    ["navicust/megfldr1"] = require("@self/megfldr1"),
    ["navicust/megfldr2"] = require("@self/megfldr2"),
    ["navicust/millions"] = require("@self/millions"),
    ["navicust/numbropn"] = require("@self/numbropn"),
    ["navicust/oilbody"] = require("@self/oilbody"),
    ["navicust/poem"] = require("@self/poem"),
    ["navicust/reflect"] = require("@self/reflect"),
    ["navicust/rush"] = require("@self/rush"),
    ["navicust/shield"] = require("@self/shield"),
    ["navicust/sliprunr"] = require("@self/sliprunr"),
    ["navicust/sneakrun"] = require("@self/sneakrun"),
    ["navicust/speed-1"] = require("@self/speed-1"),
    ["navicust/speedmax"] = require("@self/speedmax"),
    ["navicust/suprarmr"] = require("@self/suprarmr"),
    ["navicust/tango"] = require("@self/tango"),
    ["navicust/undersht"] = require("@self/undersht"),
}
