--!strict
-- Tango (EXE4's actor object #0x4D, 0x080CA9B4): the shared Tango (@exelib/supports/tango, EXE6's code) in EXE4's look,
-- the NaviCust support that heals her navi once, in link battles, when its HP drops to a quarter (0x0800C7D8), throwing
-- her heal (./heal). EXE4's own: she starts on her navi's row (0x080CAB88 adds no pixel to Y). See
-- docs/engine/chips.md §2.10.

local common = require("@exelib/supports/tango")
local heal = require("./heal")

local PUFF = asset.sprite("puff")

return common.make {
    id = "tango",
    sprite = asset.sprite("tango"),
    arrive_sound = asset.sound("arrive"),
    land_sound = asset.sound("tango-land"),
    throw_sound = asset.sound("minibomb-throw"),
    -- Her arrival sparkle (effect row 22, 0x080E1698); she leaves in a puff (row 21): the puff's animations 3 and 2.
    arrive = new.effect { sprite = PUFF, anim = 3, palette = 0 },
    puff = new.effect { sprite = PUFF, anim = 2, palette = 0 },
    heal = heal,
    pixel_nudge = false,
}
