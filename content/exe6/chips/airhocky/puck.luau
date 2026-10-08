--!strict
-- A hockey puck (the original's attack object #0x2E, `sub_80C9824`):
-- AirHocky's and PitHocky's, the shared puck (@exelib/airhocky/puck) with
-- EXE6's look. It slides diagonally from panel center to panel center,
-- bouncing off the field's sides, for its variant's number of steps;
-- PitHocky's leaves an afterimage trail.

local collision = require("../../rules/collision")
local common = require("@exelib/airhocky/puck")
local effects = require("../../lib/effects")
local sparks = require("../../lib/sparks")

local puck = {}

-- A puck's variant (a row of `byte_80C9818`, which its chip defines): the
-- steps it lasts (0: no limit), how it bounces (1: off its own side too,
-- once across), the ticks per step, and its trail's tint if it leaves one.
export type Variant = PuckVariant

puck.variant = common.variant

puck.kind = common.make_kind("airhocky/puck", {
    sprite = asset.sprite("hockey-puck"),
    sound = asset.sound("hockey-puck"),
    bounce = asset.sound("bounce"),
    is = collision.breaking,
    hits = collision.hits_navis,
    spark = sparks.breaking,
    burst = effects.puff,
})

-- `sub_80C9BA4`: a puck of `variant` for `owner` on (x, y) at height `z`,
-- with the damage word `damage`, going down the field or up.
function puck.spawn(
    owner: Object,
    x: number,
    y: number,
    element: number,
    z: number,
    damage: number,
    variant: Variant,
    down: boolean
): Object?
    return common.spawn(puck.kind, owner, x, y, element, z, damage, variant, down)
end

return puck
