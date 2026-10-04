--!strict
-- BN5's plus chips used on their own (action 0x1A's effect 3, 0x0800E198, BN6's `sub_8010488`; its parameters the
-- record's +0x10 and +0x11: the side's bonus it goes to, and whether it goes to the next chip). BN6's tests the
-- attack's special source; BN5's tests battle flag 0x40 (the per-player gauges, never in a netbattle) and adds the
-- damage alone to the side's bonus, playing the bonus sound only for a chip that goes to the next chip. Otherwise,
-- as BN6's, the sparkle by the navi (@common/instant/sparkle: BN5's bubble, the sound BN6's number).
--
-- Attck+10 and Attck+30 are `instant = plus.attack` (+0x10 0, +0x11 0: none goes to the next chip); Navi+20 is
-- `plus.navi` (+0x10 1: the side's Navi+ bonus); DarkPlus is `plus.dark`
-- (+0x11 1): its damage becomes the Atk+ bonus of the hand's next chip if that does damage, the navi takes its tint
-- (AIData+0x3C 24, which nothing counts down: its status shader goes to its first green, 0x080136B8) and the bonus
-- sound plays; else the sparkle.

local rising_bubble = require("../objects/rising-bubble/rising_bubble")
local sparkle = require("@common/instant/sparkle")

local plus = {}

-- The sparkle by the navi.
plus.sparkle = sparkle.make(rising_bubble.kind, asset.sound("bn5:panel-poison"))

-- The invalid chip's `instant` hook (action 0x1A's effect 0): the sparkle, whatever the source.
function plus.sparkle_alone(user: Object, _spec: InstantChipSpec)
    plus.sparkle(user)
end

local BONUS_SOUND = asset.sound("bn5:bonus")
-- DarkPlus's tint.
local TINT = 24

-- The side's special chip bonuses (`battle.add_special_bonus`'s index).
local ATTACK_BONUS, NAVI_BONUS = 0, 1

-- Attck+'s `instant` hook: from a special battle the damage into the side's attack bonus, else the sparkle.
function plus.attack(user: Object, _spec: InstantChipSpec)
    if battle.per_player_gauges() then
        battle.add_special_bonus(user.alliance, ATTACK_BONUS, user.attack_damage)
        return
    end
    plus.sparkle(user)
end

-- Navi+'s `instant` hook: from a special battle the damage into the side's Navi+ bonus, else the sparkle.
function plus.navi(user: Object, _spec: InstantChipSpec)
    if battle.per_player_gauges() then
        battle.add_special_bonus(user.alliance, NAVI_BONUS, user.attack_damage)
        return
    end
    plus.sparkle(user)
end

-- DarkPlus's `instant` hook (0x0800E198 with +0x11 1).
function plus.dark(user: Object, _spec: InstantChipSpec)
    if battle.per_player_gauges() then
        battle.add_special_bonus(user.alliance, ATTACK_BONUS, user.attack_damage)
        user.plus_tint = TINT
        battle.play_sound(BONUS_SOUND)
        return
    end
    local side = user.alliance
    local i = battle.hand_cursor(side)
    if battle.hand_chip_damages(side, i) then
        battle.set_hand_attack_bonus(side, i, user.attack_damage)
        user.plus_tint = TINT
        battle.play_sound(BONUS_SOUND)
        return
    end
    plus.sparkle(user)
end

return plus
