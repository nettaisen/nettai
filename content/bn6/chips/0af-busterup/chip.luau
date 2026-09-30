--!strict
-- BusterUp: instant chip effect 10 (`sub_8010820`, `off_80EC3F0[10]`),
-- run once by action 0x1C. The buster's attack level rises by the chip's
-- first parameter (to 9 at most once it reaches 10), with an effect at
-- the navi's attach point 0x20.

local buster_up = {}

-- Where the effect shows, and the effect.
local ATTACH_POINT, EFFECT = 0x20, 0x31

local SOUND = 0x119

-- `sub_8010820`.
function buster_up.instant_chip(user: Object, spec: InstantChipSpec)
    local stats = battle.navi(user.alliance)
    local level = stats.attack + spec.params[1]
    if level >= 10 then
        level = 9
    end
    stats.attack = level
    -- sub_8018810
    local dx, dz = user:attach_point(ATTACH_POINT)
    local p = user.pos
    battle.effect(Vec3.new(p.x + int.shl(dx, 16), p.y, p.z + int.shl(dz, 16)), EFFECT)
    battle.play_sound(SOUND)
end

return buster_up
