--!strict
-- Rock debris (the original's effect object #0x38, `sub_80E46D8`): a chunk
-- thrown up when a rock (./rock) breaks, which flies off with a random
-- speed and puffs into dust where it lands; the stages' boulder
-- (objects/boulder) and the encased bubble (objects/encased-bubble) throw
-- it up too. Only drawn, but it draws the simulation RNG twice. The
-- behavior is BN5's too (@common/rock/debris), made here with BN6's look.

local common = require("@common/rock/debris")
local effects = require("../../lib/effects")

local debris = {}

-- Its chunks' sheet, and the dust it lands in (effect 1, the splash's
-- look).
debris.kind = common.make_kind("bn6:rockcube/debris", asset.sprite("bn6:rock-debris"), effects.splash)

-- `sub_80E47A4`: a chunk at `pos` in sprite palette `palette`.
function debris.spawn(pos: Vec3, palette: number): Object?
    return common.spawn(debris.kind, pos, palette)
end

return debris
