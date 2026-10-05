--!strict
-- NumberSoul's hand of ten chips on EXE5's custom screen (docs/design/exe5-map.md §15.8; 0x08025BE4, the screen's
-- hand size as it opens): MegaMan in NumberSoul is dealt ten chips, whatever his custom level, in battle modes 0 and
-- 6. The soul is the one the emotion routine leaves in r1 (NaviStats +0x2C), which in battle mode 1 is the stats'
-- address instead (0x080127C0): there, and for any other soul, the hand is the custom level's (NaviStats +0x0A: the
-- framework's answer). (In a mode other than 0, 1 and 6 the routine deals five whatever the soul or the level: no
-- battle the port plays is of one, and nothing answers that yet.)

local hand = {}

-- The soul's number (NaviStats +0x2C), and its hand.
local NUMBERSOUL, CHIPS = 9, 10

-- The systems' `custom.hand_size`: NumberSoul's ten; none for anything else.
function hand.size(side: number): number?
    local soul = battle.navi(side).form.soul
    local mode = battle.mode()
    if soul ~= nil and soul.number == NUMBERSOUL and (mode == 0 or mode == 6) then
        return CHIPS
    end
    return nil
end

return hand
