--!strict
-- Instant chip effect 6 (`sub_801050C`, `off_80EC3F0[6]`): each obstacle
-- the user's side owns that still has HP is repaired to full, with an
-- effect where it stands.
--
-- No chip, weapon routine or link navi names subtype 6: nothing registers
-- this module in BN6.

local repair = {}

local EFFECT, SOUND = 6, 0x8A

-- `sub_801050C`.
function repair.instant_chip(user: Object, _spec: InstantChipSpec)
    for _, o in field.objects(user.alliance) do
        if o.hp ~= 0 then
            o.hp = o.max_hp
            battle.effect(o.pos, EFFECT)
            battle.play_sound(SOUND)
        end
    end
end

return repair
