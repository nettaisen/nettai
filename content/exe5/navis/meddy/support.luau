--!strict
-- Meddy's party support, HealPulse (EXE5 DS: 0x02003BD8, from its fighting state 0x24, 0x0208E1F8, as a custom screen is
-- asked for in the fight, each side; docs/design/exe5ds-map.md §4.7): with the light test passed and Meddy a benched
-- member who may support, the navi on the field below its maximum HP heals a twentieth of it (worried, it calms down),
-- with the heal's sparkle and sound (the DS also lights her button on the bottom screen); the flow holds 50 ticks
-- before the custom screen. No dimming, no telop.

local effects = require("../../lib/effects")
local meddy = require("../meddy").meddy
local party = require("../../rules/party")

local HOLD = 0x32
local HEAL_SOUND = asset.sound("heal")

local support = {}

-- The rules' `custom_hold`: the ticks the flow holds, if she healed.
function support.custom_hold(side: number): number?
    if not party.light(side) or not party.may_support(side, meddy) or not party.benched(side, meddy) then
        return nil
    end
    local navi = battle.player(side)
    if navi == nil or navi.hp >= navi.max_hp then
        return nil
    end
    navi.hp = math.min(navi.hp + navi.max_hp * 5 // 100, navi.max_hp)
    if battle.emotion(side) == "worried" then
        battle.set_mood(side, 0x80)
    end
    local o = battle.effect(navi.pos, effects.heal)
    if o then
        o.run_while_paused = true
    end
    battle.play_sound(HEAL_SOUND)
    return HOLD
end

return support
