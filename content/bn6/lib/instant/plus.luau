--!strict
-- The plus chips used on their own (instant chip effect 3, `sub_8010488`,
-- run once by action 0x1C): from a special source (battle flag 0x40 mode)
-- the chip's damage, and an extra, go into the bonus its side stores for
-- its special chip; otherwise a sparkle (objects/rising-bubble) rises by
-- the navi. On the custom screen a plus chip folds into the chip before it
-- (its `modifier`); this is its use alone.
--
-- Atk+10, FinalGun and NumTrap are `instant = plus.attack`, Navi+20
-- `plus.navi`, WhiCapsl `plus.attack_with(plus.PARALYZE)`. The pack's
-- records of the effect that the ruleset or a Program Advance still names
-- by number (MegaBstr, Atk+10's record, Atk+30, Uninstll, DarkPlus) run
-- `plus.by_record` through chips/0c0-atk-10. The BeastOut chip's effect is
-- the sparkle alone.

local rising_bubble = require("../../objects/rising-bubble/rising_bubble")

local plus = {}

-- The sparkle: 4 pixels toward the enemy's side, 48 up; and its sound.
local TOWARD, HEIGHT = 4, 48
local SPARKLE_SOUND = asset.sound("panel-poison")

local BONUS_SOUND = asset.sound("bonus")

-- The side's special chip bonuses (`battle.add_special_bonus`'s index).
local ATTACK_BONUS, NAVI_BONUS = 0, 1

-- The damage word's flag bits a modifier chip's bonus carries (the
-- extra, its record's third and fourth parameter bytes): WhiCapsl's
-- paralysis and Uninstll's uninstall.
plus.PARALYZE, plus.UNINSTALL = 0x4000, 0x2000

-- `loc_80104B6` (shared with `sub_80104E0`): a sparkle by the navi's
-- panel, on the enemy's side of it.
function plus.sparkle(user: Object)
    local x, y = field.center(user.panel_x, user.panel_y)
    local toward = 1 - 2 * user.alliance
    rising_bubble.spawn(Vec3.new(x + int.shl(toward * TOWARD, 16), y, int.shl(HEIGHT, 16)), 0)
    battle.play_sound(SPARKLE_SOUND)
end

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

-- A record of the effect: the bonus its first parameter names, and the
-- extra in its third and fourth (the upper halfword).
function plus.by_record(user: Object, spec: InstantChipSpec)
    local p = spec.params
    effect(user, p[1], bit32.bor(p[3], bit32.lshift(p[4], 8)))
end

return plus
