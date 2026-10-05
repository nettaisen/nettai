--!strict
-- BassAnly's shot (the original's attack object #0xC1, `sub_80DD34C`): the
-- shared dark ball (@exelib/bassanly/shot), EXE6's kind: its sprite and
-- sound, thrown, hitting navis. What it is spawned with is a `DarkBall`
-- (types.d.luau; the original's Param1 to Param3). See
-- docs/engine/chips.md §3.6.35.

local collision = require("../../rules/collision")
local common = require("@exelib/bassanly/shot")

local shot = {}

shot.kind = common.make_kind {
    id = "bassanly/shot",
    sprite = asset.sprite("bass-anly-shot"),
    sound = asset.sound("bass-anly-shot"),
    self_type = collision.thrown,
    target = collision.hits_navis,
}

-- `sub_80DD594` (+ `sub_80DD5B4`: it runs while dimmed): a shot from `owner`
-- on (x, y) with `element`, the damage word and what it is. `z` is the
-- spawner's register that becomes its Z.
function shot.spawn(
    owner: Object,
    x: number,
    y: number,
    element: number,
    what: DarkBall,
    damage: number,
    z: number
): Object?
    return common.spawn(shot.kind, owner, x, y, element, what, damage, z)
end

return shot
