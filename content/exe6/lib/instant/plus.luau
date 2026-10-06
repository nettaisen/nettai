--!strict
-- The plus chips used on their own (instant chip effect 3, `sub_8010488`,
-- run once by action 0x1C): from a special source (the chip gate battle)
-- the chip's damage, and an extra, go into the bonus its side stores for
-- its special chip; otherwise a sparkle (@exelib/instant/sparkle, EXE6's
-- bubble) rises by the navi. On the custom screen a plus chip folds into the chip before it
-- (its `modifier`); this is its use alone.
--
-- Atk+10, Atk+30, FinalGun, NumTrap, MegaBstr and DarkPlus are `instant =
-- plus.attack`, Navi+20 `plus.navi`, WhiCapsl
-- `plus.attack_with(plus.PARALYZE)` and Uninstll
-- `plus.attack_with(plus.UNINSTALL)`. The BeastOut chip's effect and the
-- invalid chip's are the sparkle alone (`plus.sparkle_alone`).

local rising_bubble = require("../../objects/rising_bubble")
local sparkle = require("@exelib/instant/sparkle")

local plus = {}

local BONUS_SOUND = asset.sound("bonus")

-- The side's special chip bonuses (`battle.add_special_bonus`'s index).
local ATTACK_BONUS, NAVI_BONUS = 0, 1

-- The damage word's flag bits a modifier chip's bonus carries (the
-- extra, its record's third and fourth parameter bytes): WhiCapsl's
-- paralysis and Uninstll's uninstall.
plus.PARALYZE, plus.UNINSTALL = 0x4000, 0x2000

-- `loc_80104B6` (shared with `sub_80104E0`): a sparkle by the navi's
-- panel, on the enemy's side of it.
plus.sparkle = sparkle.make(rising_bubble.kind, asset.sound("panel-poison"))

-- `sub_8010488`: from a special source, the attack's damage plus `extra`
-- into the side's bonus `bonus`; else the sparkle.
local function effect(user: Object, bonus: number, extra: number)
    if user.special_source ~= 0 then
        battle.add_special_bonus(user.alliance, bonus, user.attack_damage + extra)
        battle.play_sound(BONUS_SOUND)
        return
    end
    plus.sparkle(user)
end

-- Atk+'s `instant` hook: the side's attack bonus.
function plus.attack(user: Object, _spec: InstantChipSpec)
    effect(user, ATTACK_BONUS, 0)
end

-- Navi+'s `instant` hook: the side's navi bonus.
function plus.navi(user: Object, _spec: InstantChipSpec)
    effect(user, NAVI_BONUS, 0)
end

-- A modifier chip's `instant` hook: the side's attack bonus, with the
-- damage word's flag bits `flags` (`plus.PARALYZE`, `plus.UNINSTALL`).
function plus.attack_with(flags: number): (user: Object, spec: InstantChipSpec) -> ()
    return function(user: Object, _spec: InstantChipSpec)
        effect(user, ATTACK_BONUS, flags)
    end
end

-- The BeastOut chip's `instant` hook, and the invalid chip's (instant chip
-- effect 0, `sub_80104E0`): the sparkle, whatever the source.
function plus.sparkle_alone(user: Object, _spec: InstantChipSpec)
    plus.sparkle(user)
end

return plus
