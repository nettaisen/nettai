--!strict
-- Instant chip effect 2 (`sub_8010474`, `off_80EC3F0[2]`): the navi turns
-- invisible for a number of ticks (the flash timer, CollisionData+0x24,
-- with the invisible flag), with a sound. The game takes the ticks from
-- the attack's parameters' low halfword; here they are the builder's
-- argument.
--
-- No chip has effect 2: weapon routine 0x71 (`sub_8012464`, a link
-- navi's) names it, and calls `invisible.instant(ticks)` when it is
-- ported.

local invisible = {}

local SOUND = asset.sound("invisible")

-- `sub_8010474`: an `instant` hook that hides the user for `ticks` ticks.
function invisible.instant(ticks: number): (user: Object, spec: InstantChipSpec) -> ()
    return function(user: Object, _spec: InstantChipSpec)
        user:set_status_timer("flash", ticks)
        user:set_status("invisible", true)
        battle.play_sound(SOUND)
    end
end

return invisible
