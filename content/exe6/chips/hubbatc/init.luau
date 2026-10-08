--!strict
-- HubBatc: the navi gets the Hub batch's programs for the rest of the
-- battle: a stronger buster, a bigger custom screen, a B+Back shield, the
-- shoes and the undershirt (its change, `sub_80E95D8`, on the navi-changing
-- chips' controller: lib/navi_boost/controller).

local collision = require("../rules/collision")
local navi_boost = require("../lib/navi_boost/controller")
local shield = require("../navis/megaman/weapons/shield")

-- The effect around the navi (the original's effect 0x5A) and its sound;
-- when the log-in sound plays, by timer.
local HUB_EFFECT = new.effect { sprite = asset.sprite("hub"), anim = 0, palette = 0 }
local HUB_SOUND = asset.sound("hub")
local LOG_IN_AT = 0x1C
-- The buster, custom screen and B+Back special (the NaviCust Shield) it
-- gives.
local MIN_BUSTER, RAPID, CHARGE, CUSTOM = 5, 4, 4, 8
local BACK_SPECIAL = shield.shield

-- `sub_80E95D8`.
local HUB_BATCH = navi_boost.change(function(me: Object)
    if me.phase == 0 then
        me.phase = 1
        me.timer = navi_boost.WAIT
        local user = navi_boost.user_of(me)
        local dx, dy = navi_boost.attach_offset(me, user)
        local p = user.pos
        battle.effect(Vec3.new(p.x + dx * 0x10000, p.y, p.z + dy * 0x10000), HUB_EFFECT)
        battle.play_sound(HUB_SOUND)
        local stats = battle.navi(user.alliance)
        local buster = user:buster_damage()
        if buster < MIN_BUSTER then
            stats.attack = stats.attack + MIN_BUSTER - buster
        end
        stats.rapid = RAPID
        stats.charge = CHARGE
        stats.custom_level = CUSTOM
        local shrink = stats.hand_shrink_turn
        local turn = battle.turn()
        if shrink ~= 0 and turn >= shrink then
            stats.hand_shrink_turn = turn + 1
        end
        if not stats.back_special_weapon then
            stats.back_special_weapon = BACK_SPECIAL
        end
        user:set_status("air_shoe", true)
        user:set_status("float_shoe", true)
        user:set_status("undershirt", true)
        stats.float_shoes = true
        stats.air_shoes = true
        stats.undershirt = true
        -- sub_80107C0: bodies' hit modifier (3 in link battles); the body
        -- floats now (`sub_801A082`).
        local hit_mod = if battle.link() then 3 else 0
        user:reset_collision_types(collision.floating_navi, collision.navi_target, hit_mod)
    end
    navi_boost.wait(me, LOG_IN_AT)
end)

-- (The record's extra flags 0x08 and 0x10 have no battle reader, and no
-- name.)
local chip: Chip = {
    codes = { "J" },
    element = "null", family = "null", class = "giga",
    rarity = 4, mb = 99, damage = 0, hit_param = 148, lockout = 0,
    flags = { "dimming", "library" },
    slot_in_limit = 1,
    dimming = navi_boost.hook(HUB_BATCH),
}

return { hubbatc = chip }
