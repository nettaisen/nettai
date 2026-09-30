--!strict
-- SyncTrgr: instant chip effect 13 (`sub_80EC44C`, `off_80EC3F0[13]`),
-- run once by action 0x1C. The navi's mood goes to the top: Full Synchro
-- (unless its emotion is held).

local sync_trigger = {}

local SOUND = 0x8D

-- The mood Full Synchro starts from.
local FULL_SYNCHRO = 0xFF

-- `sub_80EC44C`.
function sync_trigger.instant_chip(user: Object, _spec: InstantChipSpec)
    battle.play_sound(SOUND)
    -- sub_8015BEC
    battle.set_mood(user.alliance, FULL_SYNCHRO)
end

return sync_trigger
