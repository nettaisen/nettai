--!strict
-- CrakOut, DublCrak and TripCrak (EXE5 chips 0x05B to 0x05D): their action (0x1B, 0x080EC79C) and the crack they leave
-- (attack object 0x18, 0x080C7704), as EXE5 and EXE4 share them (@exelib/crack), with EXE5's look: the navi points
-- (animation 12), the crack's dust and sounds, its spawner keeping to the field's panels (0x080C77C2).

local collision = require("../../rules/collision")
local common = require("@exelib/crack")

local crack = {}

local CRACK = common.make {
    id = "crakout/crack",
    anim = 12,
    sprite = asset.sprite("dust"),
    sound = asset.sound("bblstar1"),
    -- `object_breakPanelLoud` (EXE5's 0x0800B204): the panel break with its own sound (0xDA).
    break_sound = asset.sound("crack-shot"),
    is = collision.thrown,
    hits = collision.hits_navis,
    on_field_only = true,
}

-- The action of a chip that cracks the panels of `panels` (its pattern, a row of 0x080C7818 by its subtype: { dx, dy }
-- from the panel ahead, dx toward the navi's front).
function crack.action(panels: { { number } }): Action
    return common.action(CRACK, panels)
end

return crack
