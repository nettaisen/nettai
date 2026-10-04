--!strict
-- A hockey puck (the original's attack object #0x2E, `sub_80C9824`):
-- AirHocky's and PitHocky's, the shared puck (@exelib/airhocky/puck) with
-- BN6's look. It slides diagonally from panel center to panel center,
-- bouncing off the field's sides, for its variant's number of steps;
-- PitHocky's leaves an afterimage trail.

local collision = require("../../rules/collision")
local common = require("@exelib/airhocky/puck")
local effects = require("../../lib/effects")
local sparks = require("../../lib/sparks")

local puck = {}

-- A puck's variant (a row of `byte_80C9818`).
export type Variant = PuckVariant

puck.variant = common.variant

-- `byte_80C9818`'s rows: the steps it lasts (0: no limit), how it bounces
-- (1: off its own side too, once across), and the ticks per step; the
-- trail is PitHocky's. The first two rows no chip has.
local ROWS: { [number]: Variant } = {
    [0] = puck.variant { steps = 6, own_side = 1, ticks = 6 },
    [1] = puck.variant { steps = 0x0A, own_side = 1, ticks = 6 },
    [2] = puck.variant { steps = 0x0A, own_side = 1, ticks = 6 },
    [3] = puck.variant { steps = 0x12, own_side = 1, ticks = 6, trail = true },
}
-- The chips' pucks.
puck.variants = { airhocky = ROWS[2], pithocky = ROWS[3] }

puck.kind = common.make_kind("bn6:airhocky/puck", {
    sprite = asset.sprite("bn6:hockey-puck"),
    sound = asset.sound("bn6:hockey-puck"),
    bounce = asset.sound("bn6:bounce"),
    is = collision.breaking,
    hits = collision.hits_navis,
    spark = sparks.breaking,
    burst = effects.puff,
    -- PitHocky's trail, tinted.
    trail_tint = 0x8318,
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
