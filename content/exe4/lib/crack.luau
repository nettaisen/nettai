--!strict
-- EXE4's CrakOut and its kin, the chips' action 0x21 (0x080ECA14) and the crack it leaves (attack object 0x18,
-- 0x080CFE5C): @exelib/crack (EXE5's code, emap: the crack's routines the same, the action's and the spawner's
-- similar) with EXE4's look: the navi points in animation 16; the crack's dust (14-0E), its sound as it comes (0x1BA)
-- and the break's (0x08009D04: 0x1A9); its spawner (0x080CFF1A) places a crack on every panel of the variant's
-- pattern, on the field or not. Each chip composes its own from its variant's pattern (the rows at 0x080CFF64).

local collision = require("../rules/collision")
local common = require("@exelib/crack")

local crack = {}

local CRACK = common.make {
    id = "crack",
    anim = 16,
    sprite = asset.sprite("dust"),
    sound = asset.sound("bblstar1"),
    break_sound = asset.sound("crack-shot"),
    is = collision.thrown,
    hits = collision.hits_navis,
    on_field_only = false,
}

-- The patterns by variant (0x080CFF70, 0x080CFF73, 0x080CFF78: { dx, dy } from the panel ahead, dx toward the navi's
-- front): the panel ahead; it and the one past it; it and the ones above and below.
crack.ahead = { { 0, 0 } }
crack.ahead_two = { { 0, 0 }, { 1, 0 } }
crack.column = { { 0, 0 }, { 0, -1 }, { 0, 1 } }

export type CrackSpec = {
    id: string,
    panels: { { number } },
}

-- The chips' action 0x21 with a variant's pattern.
function crack.action(spec: CrackSpec): Action
    return common.action(CRACK, spec.panels, spec.id)
end

return crack
