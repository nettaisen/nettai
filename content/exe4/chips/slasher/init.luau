--!strict
-- Slasher (EXE4 chip 0x038; its record at 0x080197EC + 0x38 * 0x2C): action 0x35 (0x080ED572: EXE4's own; EXE5's
-- action 0x29 grew from it). The navi stands (animation 0) and, while A is held, waits for an enemy navi to stand on
-- its side's area; then (or at once, if one already does) it slashes that one's column: animation 5 (its load not
-- forced), a counter window, the blade (lib/swords') with the swing's sound, Slasher's name shown to the other player
-- again (0x080164B4, its damage and bonus), and 9 ticks later a wide slash on the column's middle row (the region word
-- 0x0705FF04, hit modifier 3, 16 pixels up) with the wide slash's effect (row 0x17); 22 ticks; then 6 more, and back to
-- idle without the blade. Letting go of A first ends it. After each phase a trap's counter (requests 0x200, 0x400)
-- takes over. EXE5's but: no invulnerability while it slashes, no stun strike.
--
-- The column (0x0800D432, EXE5's 0x0800FEC4): from its side's back column forward, the first whose panel holds the
-- other side's navi on its own side's area (side 0: an other-navi panel not of side 1; side 1: one of side 1;
-- 0x0800D474), bottom row up.
-- Its record is written by tools/exe4/gen_content.py (bn6battle-verify) from the EXE4 ROMs.

local attachment = require("../objects/attachment")
local collision = require("../rules/collision")
local regions = require("@exelib/regions")
local swords = require("../lib/swords")

local WIDE_SLASH = new.effect { sprite = asset.sprite("sword-slash"), anim = 0, palette = 0 }
local HIT = { hits = collision.hits_navis, is = collision.slash }

type State = {
    -- Ticks left in the slash (AIAttackVars +0x10), and the column it slashes (+0x16; its row, +0x17, is 2).
    timer: number,
    target_x: number,
}

local START, WAIT, SLASH, RECOVER = 0x0, 0x4, 0x8, 0xC
-- What the column search looks for, by side (0x0800D474): a panel's flags required and forbidden.
local SEARCH = {
    [0] = { require = 0x00200000, forbid = 0x00000020 },
    [1] = { require = 0x00400020, forbid = 0x00000000 },
}
local ROW = 2
local Z = 0x10_0000
local SLASH_TICKS, SLASH_AT, RECOVER_TICKS = 0x15, 0x0C, 5
-- The navi's number: the blade animates by it (NaviStats +0x23, MegaMan's 0).
local NAVI_NUMBER = 0

-- 0x0800D432: the column, or 0: from the side's back column (0x0800D484: 1 or 6) toward its facing.
local function find(me: Object): number
    local s = SEARCH[me.alliance]
    local x = 1 + 5 * me.alliance
    while true do
        for y = 3, 1, -1 do
            if field.check(x, y, s.require, s.forbid) then
                return x
            end
        end
        x += me.facing
        if x - 1 < 0 or x - 1 > 6 then
            return 0
        end
    end
end

local function update(me: Object, s: State)
    local step = me.step
    if step == START then
        -- 0x080ED5AC: standing; at once, if the column is there.
        me:set_animation(0)
        local x = find(me)
        if x ~= 0 then
            s.target_x = x
            me.step, me.step_init = SLASH, 0
        else
            me.step, me.step_init = WAIT, 0
        end
    elseif step == WAIT then
        -- 0x080ED5D0: while A is held, wait for it.
        if me:held("a") then
            local x = find(me)
            if x ~= 0 then
                s.target_x = x
                me.step, me.step_init = SLASH, 0
            end
        else
            me:exit_attack()
        end
    elseif step == SLASH then
        -- 0x080ED5F6: the slash on the 10th tick of 22.
        if me.step_init == 0 then
            me.anim = 5
            me:open_counter_window()
            battle.play_sound(swords.swing)
            attachment.attach(me, swords.blade, "related", NAVI_NUMBER)
            me:set_status("using_action", true)
            s.timer = SLASH_TICKS
            -- (Chip 0x38, Slasher, whose action this is alone.)
            local chip = me.attack_chip
            if chip == nil then
                error("Slasher's slash names chip 0x38: its action runs without the chip")
            end
            battle.show_used_chip(me.alliance, chip, me.attack_damage, me.extra)
            me.step_init = 4
        end
        if s.timer == SLASH_AT then
            local word = int.u32(me.attack_damage + me.hit_param * 0x10000 + me.extra)
            battle.hitbox(me, {
                panel_x = s.target_x,
                panel_y = ROW,
                element = me.attack_element,
                z = Z,
                region = regions.wide,
                hit_effect = nil,
                target = HIT.hits,
                self_type = HIT.is,
                damage = bit32.band(word, 0xFFFF),
                stamina = bit32.rshift(word, 16),
                hit_mod = 3,
            })
            local cx, cy = field.center(s.target_x, ROW)
            battle.effect(Vec3.new(cx, cy, Z), WIDE_SLASH, me.alliance)
        end
        local t = s.timer - 1
        s.timer = int.u16(t)
        if t < 0 then
            me.step, me.step_init = RECOVER, 0
        end
    elseif step == RECOVER then
        -- 0x080ED69C: 6 ticks, then back to idle without the blade.
        if me.step_init == 0 then
            s.timer = RECOVER_TICKS
            me.step_init = 4
        end
        local t = s.timer - 1
        s.timer = int.u16(t)
        if t < 0 then
            me.related1 = nil
            me:exit_attack()
        end
    else
        error(string.format("Slasher's phase %#x reads past its table (0x080ED598)", step))
    end
    if me:request("anti_damage_triggered") or me:request("anti_sword_triggered") then
        me:check_reactive_abort()
    end
end

local slasher: Chip = {
    action = new.action { id = "slasher/action", state = { timer = "u16", target_x = "u8" }, update = update },
    codes = { "F", "H", "R" },
    element = "null",
    family = "sword",
    class = "standard",
    rarity = 2,
    mb = 18,
    damage = 240,
    hit_param = 40,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
}

return { slasher = slasher }
