--!strict
-- Instant chip effect 6 (`sub_801050C`, `off_80EC3F0[6]`): each of the
-- user's side's field objects (its three registry slots) that still has HP
-- is repaired to full, with the heal effect where it stands and the heal
-- sound.
--
-- No chip, weapon routine or link navi of BN6's names effect 6: nothing
-- uses `repair.instant` (unverified: no scenario reaches it).

local repair = {}

local HEAL = define.effect { sprite = asset.sprite("bn6:heal"), anim = 0, palette = 0 }
local SOUND = asset.sound("bn6:heal")

-- `sub_801050C`: an `instant` hook.
function repair.instant(user: Object, _spec: InstantChipSpec)
    for _, o in field.objects(user.alliance) do
        if o.hp ~= 0 then
            o.hp = o.max_hp
            battle.effect(o.pos, HEAL)
            battle.play_sound(SOUND)
        end
    end
end

return repair
