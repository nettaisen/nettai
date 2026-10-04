--!strict
-- Tango (BN5's actor object #0x4D, 0x080C2714): the shared Tango (@exelib/supports/tango, BN6's code) in BN5's look,
-- the NaviCust support that heals her navi once, in link battles, when its HP drops to a quarter (0x0800E3B6), throwing
-- her heal (./heal). See docs/engine/chips.md §2.10.

local common = require("@exelib/supports/tango")
local heal = require("./heal")

local PUFF = asset.sprite("puff")

return common.make {
    id = "tango",
    sprite = asset.sprite("tango"),
    arrive_sound = asset.sound("arrive"),
    land_sound = asset.sound("tango-land"),
    throw_sound = asset.sound("minibomb-throw"),
    -- Her arrival sparkle (BN5's effect 0x15); she leaves in a puff (effect 0x14: the rows of BN5's 0x080DFC94 are
    -- BN6's).
    arrive = define.effect { sprite = PUFF, anim = 3, palette = 0 },
    puff = define.effect { sprite = PUFF, anim = 2, palette = 0 },
    heal = heal,
}
