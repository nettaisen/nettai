--!strict
-- BassAnly's shot (EXE4's attack object #0x73, 0x080DB7E8): the shared dark ball (@exelib/bassanly/shot), EXE4's kind:
-- Bass's sheet 08-19 (its animations 33 flying and 34 bursting, its afterimages the same sheet), its sound, thrown,
-- hitting navis, and level 2's ticks (BassAnly throws level 2: 0x080CB304): its push 0x1C and twice 2, its diagonal
-- steps 12 less twice 2 (0x080DBA0C).

local collision = require("../../rules/collision")
local common = require("@exelib/bassanly/shot")

local shot = {}

shot.kind = common.make_kind {
    id = "bassanly/shot",
    sprite = asset.sprite("bass"),
    anim = 33,
    sound = asset.sound("bass-anly-shot"),
    self_type = collision.thrown,
    target = collision.hits_navis,
    push_ticks = 0x1C + 2 * 2,
    step_ticks = 0xC - 2 * 2,
}

-- 0x080DBA2C (+ 0x080DBA4C: it runs while dimmed): a shot from `owner` on (x, y) with `element`, the damage word and
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
