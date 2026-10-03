--!strict
-- BN5's plus chips used on their own (action 0x1A's effect 3, 0x0800E198, BN6's `sub_8010488`; its parameters the
-- record's +0x10 and +0x11: the side's bonus it goes to, and whether it goes to the next chip). BN6's tests the
-- attack's special source; BN5's tests battle flag 0x40 (the per-player gauges, never in a netbattle) and adds the
-- damage alone to the side's bonus, playing the bonus sound only for a chip that goes to the next chip. Otherwise,
-- as BN6's, the sparkle by the navi (@bn6/lib/instant/plus).
--
-- Attck+10 and Attck+30 are `instant = plus.attack` (+0x11 0: none goes to the next chip; DarkPlus's does, with
-- its own module).

local bn6_plus = require("@bn6/lib/instant/plus")

local plus = {}

-- The side's special chip bonuses (`battle.add_special_bonus`'s index).
local ATTACK_BONUS = 0

-- Attck+'s `instant` hook: from a special battle the damage into the side's attack bonus, else the sparkle.
function plus.attack(user: Object, _spec: InstantChipSpec)
    if battle.per_player_gauges() then
        battle.add_special_bonus(user.alliance, ATTACK_BONUS, user.attack_damage)
        return
    end
    bn6_plus.sparkle(user)
end

return plus
