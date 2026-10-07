--!strict
-- EXE4's plus chips (the spawners' action 0x20, its variant 4: 0x0800C6D2; Atk+10's, Navi+20's, Atk+30's and the
-- invalid chip's): in a link battle (0x080406A4: event flag 0x1187, which every link battle has set) the attack's
-- damage goes to the side's bonus its parameter +0x10 names (the side's chip block +0x36 + 2 * the parameter: 0 the
-- Atk+ bonus, 1 the Navi+ bonus); else the sparkle by the navi (@exelib/instant/sparkle: a rising bubble 4 pixels
-- toward the enemy's side and 48 up, 0x080E387A) and its sound (0x124). As EXE5's (its lib/plus, 0x0800E198), which
-- tests the operation battle instead.

local rising_bubble = require("../objects/rising_bubble")
local sparkle = require("@exelib/instant/sparkle")

local plus = {}

-- The sparkle by the navi.
plus.sparkle = sparkle.make(rising_bubble.kind, asset.sound("panel-poison"))

-- The spawner of a plus chip of parameter `bonus` (its record's +0x10: the side's bonus its damage goes to).
function plus.spawner(bonus: number): (user: Object, element: number, damage: number) -> ()
    return function(user: Object, _element: number, _damage: number)
        if battle.link() then
            battle.add_special_bonus(user.alliance, bonus, user.attack_damage)
            return
        end
        plus.sparkle(user)
    end
end

return plus
