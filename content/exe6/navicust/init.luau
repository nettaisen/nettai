--!strict
-- EXE6's navicust programs (docs/design/content-model-v2.md §4.0), by id: the tables of the modules that define them
-- as one, which the game's init.luau returns as its `navicust_programs`. Written by tools/content/index.py (the
-- verification workspace) from the modules that are there: run it again rather than edit this, also to settle a merge
-- conflict in it.

local merge = require("@exelib/merge")

return merge {
    -- NaviCust programs.
    require("@self/airshoes"),
    require("@self/antidmg"),
    require("@self/attack-1"),
    require("@self/attckmax"),
    require("@self/autoheal"),
    require("@self/battery"),
    require("@self/beat"),
    require("@self/bodypack"),
    require("@self/bugstop"),
    require("@self/bustpack"),
    require("@self/charge-1"),
    require("@self/chargmax"),
    require("@self/chpshufl"),
    require("@self/collect"),
    require("@self/custom1"),
    require("@self/custom2"),
    require("@self/fish"),
    require("@self/fldrpak1"),
    require("@self/fldrpak2"),
    require("@self/flotshoe"),
    require("@self/fstbarr"),
    require("@self/gigfldr1"),
    require("@self/hp-100"),
    require("@self/hp-200"),
    require("@self/hp-300"),
    require("@self/hp-400"),
    require("@self/hp-50"),
    require("@self/hp-500"),
    require("@self/humor"),
    require("@self/jungle"),
    require("@self/megfldr1"),
    require("@self/megfldr2"),
    require("@self/millions"),
    require("@self/numbropn"),
    require("@self/oilbody"),
    require("@self/poem"),
    require("@self/reflect"),
    require("@self/rush"),
    require("@self/shield"),
    require("@self/sliprunr"),
    require("@self/sneakrun"),
    require("@self/speed-1"),
    require("@self/speedmax"),
    require("@self/suprarmr"),
    require("@self/tango"),
    require("@self/undersht"),
}
