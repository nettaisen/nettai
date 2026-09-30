--!strict
-- Instant chip effect 2 (`sub_8010474`, `off_80EC3F0[2]`): the navi turns
-- invisible for as many ticks as the attack's parameters' low halfword
-- (the flash timer, CollisionData+0x24, with the invisible flag).
--
-- No chip has subtype 2. Weapon routine 0x71 (`sub_8012464`, a link navi's)
-- names it: its weapon.toml registers this module (`instant_chip = 2`)
-- once that weapon is content.

local invisible = {}

local SOUND = 0x93

-- `sub_8010474`.
function invisible.instant_chip(user: Object, spec: InstantChipSpec)
    local ticks = bit32.bor(spec.params[1], bit32.lshift(spec.params[2], 8))
    user:set_status_timer("flash", ticks)
    user:set_status("invisible", true)
    battle.play_sound(SOUND)
end

return invisible
