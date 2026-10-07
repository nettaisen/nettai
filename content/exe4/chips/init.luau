--!strict
-- EXE4's chips (docs/design/content-model-v2.md §4.0), by id: the tables of the modules that define them as one, which
-- the game's init.luau returns as its `chips`. Written by tools/content/index.py (the verification workspace) from the
-- modules that are there: run it again rather than edit this, also to settle a merge conflict in it.

local merge = require("@exelib/merge")

return merge {
    -- Chips: a module defining one, or a series module defining several.
    require("@self/airshot"),
    require("@self/areagrab"),
    require("@self/atk-10"),
    require("@self/atk-30"),
    require("@self/bubbler"),
    require("@self/cannon"),
    require("@self/crakout"),
    require("@self/darkbomb"),
    require("@self/drkcanon"),
    require("@self/drklance"),
    require("@self/drkrecov"),
    require("@self/drkspred"),
    require("@self/drkstage"),
    require("@self/drksword"),
    require("@self/drkvulcn"),
    require("@self/heatshot"),
    require("@self/invalid"),
    require("@self/lance"),
    require("@self/minibomb"),
    require("@self/navi-20"),
    require("@self/panlgrab"),
    require("@self/recov"),
    require("@self/spreader"),
    require("@self/sword"),
    require("@self/vulcan"),
    -- A navi chip's chips, with its navi.
    require("./navis/megaman/forms/aquasoul/chip"),
    require("./navis/megaman/forms/firesoul/chip"),
    require("./navis/megaman/forms/gutssoul/chip"),
    require("./navis/megaman/forms/junksoul/chip"),
    require("./navis/megaman/forms/metalsoul/chip"),
    require("./navis/megaman/forms/numbersoul/chip"),
    require("./navis/megaman/forms/protosoul/chip"),
    require("./navis/megaman/forms/rollsoul/chip"),
    require("./navis/megaman/forms/searchsoul/chip"),
    require("./navis/megaman/forms/thundersoul/chip"),
    require("./navis/megaman/forms/windsoul/chip"),
    require("./navis/megaman/forms/woodsoul/chip"),
}
