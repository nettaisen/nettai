--!strict
-- BassAnly's shot (BN5's attack object #0x73, 0x080DECF4): the shared dark ball (@common/bassanly/shot), BN5's kind:
-- its sprite 10-4F (0x080D4470) and sound, thrown, hitting navis.

local collision = require("../../rules/collision")
local common = require("@common/bassanly/shot")

local shot = {}

shot.kind = common.make_kind {
    id = "bn5:bassanly/shot",
    sprite = asset.sprite("bn5:bass-anly-shot"),
    sound = asset.sound("bn5:bass-anly-shot"),
    self_type = collision.thrown,
    target = collision.hits_navis,
}

-- 0x080D4650 (+ 0x080D4670: it runs while dimmed): a shot from `owner` on (x, y) with `element`, the damage word and
-- what it is. `z` is the spawner's register that becomes its Z.
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
