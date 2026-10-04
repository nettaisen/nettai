--!strict
-- Tango (the original's actor object #0x4D, `sub_80C3734`): the shared Tango (@exelib/supports/tango, BN5's too) in
-- BN6's look, the NaviCust support that heals her navi once, in link battles, when its HP drops to a quarter
-- (`sub_8010660`), throwing her heal (./heal). See docs/engine/chips.md §2.10.

local common = require("@exelib/supports/tango")
local effects = require("../effects")
local heal = require("./heal")

return common.make {
    id = "tango",
    sprite = asset.sprite("tango"),
    arrive_sound = asset.sound("arrive"),
    land_sound = asset.sound("tango-land"),
    throw_sound = asset.sound("minibomb-throw"),
    -- Her arrival sparkle (the original's effect 0x15); she leaves in a puff.
    arrive = define.effect { sprite = asset.sprite("puff"), anim = 3, palette = 0 },
    puff = effects.puff,
    heal = heal,
}
