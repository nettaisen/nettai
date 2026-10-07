--!strict
-- SearchMan's scope, EXE5's (effect object #0x43, 0x080E5208): @exelib/serchman/scope with EXE5's looks (the scope,
-- and the sight that Satelity's controller, navis/searchman/controller, sends out), its sounds (0x10E as it moves,
-- which 0x080E52D4 loads as 15 + 255; the power-up's) and its sweep (0x080E5416: from the first column past its
-- user's area to the far edge).

local common = require("@exelib/serchman/scope")

return common.make {
    id = "serchman/scope",
    scope = asset.sprite("search-scope"),
    sight = asset.sprite("sight"),
    move_sound = asset.sound("erase-man"),
    lock_sound = asset.sound("mark"),
    bonus_sound = asset.sound("ok-8b"),
    columns = common.exe5_columns,
}
