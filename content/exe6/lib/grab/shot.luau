--!strict
-- A grab shot (the original's attack object #0xF, `sub_80C6414`): the
-- shared shot (@exelib/grab/shot), EXE6's kind and look.

local collision = require("../../rules/collision")
local common = require("@exelib/grab/shot")

local shot = {}

shot.kind = common.make_kind {
    id = "grab/shot",
    sprite = asset.sprite("grab-shot"),
    fall_sound = asset.sound("grab-shot-2"),
    land_sound = asset.sound("grab-shot"),
    target = collision.hits_navis,
    self_type = collision.thrown,
}

-- `sub_80C6548`: a grab shot for `owner`'s side onto (x, y), with the
-- damage word `damage`.
function shot.spawn(owner: Object, x: number, y: number, element: number, damage: number): Object?
    return common.spawn(shot.kind, owner, x, y, element, damage)
end

return shot
