--!strict
-- The plus chips used on their own: instant chip effect 3 (`sub_8010488`,
-- `off_80EC3F0[3]`), run once by action 0x1C: Atk+10, Atk+30, Navi+20,
-- MegaBstr, WhiCapsl, Uninstll, DarkPlus and the special chips of the
-- subtype. From a special source (battle flag 0x40 mode) the chip's damage
-- and its third and fourth parameters go into the side's plus bonus (its
-- first parameter: 0 Atk+, 1 Navi+) for the next chips; otherwise a
-- sparkle rises by the navi. The BeastOut chip's effect (0) is that
-- sparkle alone.

local plus_sparkle = require("../../objects/plus-sparkle/plus_sparkle")

local plus_chip = {}

-- The sparkle: 4 pixels toward the enemy's side, 48 up; and its sound.
local TOWARD, HEIGHT = 4, 48
local SPARKLE_SOUND = 0x90

local BONUS_SOUND = 0x157

-- `loc_80104B6` (shared with `sub_80104E0`): a sparkle by the navi's
-- panel, on the enemy's side of it.
function plus_chip.sparkle(user: Object)
    local x, y = field.center(user.panel_x, user.panel_y)
    local toward = 1 - 2 * user.alliance
    plus_sparkle.spawn(Vec3.new(x + int.shl(toward * TOWARD, 16), y, int.shl(HEIGHT, 16)), 0)
    battle.play_sound(SPARKLE_SOUND)
end

-- `sub_8010488`.
function plus_chip.instant_chip(user: Object, _spec: InstantChipSpec)
    if user.special_source ~= 0 then
        -- The bonus: the damage, plus the parameters' upper halfword.
        local extra = bit32.bor(user:attack_param(3), bit32.lshift(user:attack_param(4), 8))
        battle.add_plus_bonus(user.alliance, user:attack_param(1), user.attack_damage + extra)
        battle.play_sound(BONUS_SOUND)
        return
    end
    plus_chip.sparkle(user)
end

return plus_chip
