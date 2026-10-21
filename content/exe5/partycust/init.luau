--!strict
-- EXE5's partycust programs (docs/design/content-model-v2.md §4.0), by id: the tables of the modules that define them
-- as one, which the game's init.luau returns as its `partycust_programs`. Written by tools/content/index.py (the
-- verification workspace) from the modules that are there: run it again rather than edit this, also to settle a merge
-- conflict in it.

local merge = require("@exelib/merge")

return merge {
    -- PartyCust programs.
    require("@self/p_atk_1"),
    require("@self/p_atk_2"),
    require("@self/p_atk_3"),
    require("@self/p_btlpk1"),
    require("@self/p_btlpk2"),
    require("@self/p_chp_30"),
    require("@self/p_chp_40"),
    require("@self/p_chp_50"),
    require("@self/p_hp_100"),
    require("@self/p_hp_200"),
    require("@self/p_hp_300"),
    require("@self/p_hp_50"),
    require("@self/p_spport"),
}
