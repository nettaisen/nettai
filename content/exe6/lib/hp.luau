--!strict
-- Things with HP that aren't navis (a set-down FlashBomb, a BugBomb, a
-- LilBoiler...) taking their hits.

local effects = require("./effects")

local hp = {}

local SOUND_HIT_BOMB = asset.sound("hit-bomb-1")

-- `object_applyDamage`: take this tick's damage (`take_damage`, which see
-- for `mode`), and when that says it was broken (-1) or hit (1), show an
-- explosion with its sound.
function hp.apply_damage(me: Object, mode: number): number
    local r = me:take_damage(mode)
    if r ~= 0 then
        battle.effect(me.pos, effects.explosion)
        battle.play_sound(SOUND_HIT_BOMB)
    end
    return r
end

return hp
