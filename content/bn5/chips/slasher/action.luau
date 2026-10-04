--!strict
-- Slasher's action (action 0x29, 0x080ED328: BN5's own): the navi waits, standing, while A is held, for an enemy
-- navi to stand on its side's area; then (or at once, if one already does) it slashes that one's column (a wide
-- slash on its middle row, 16 pixels up, hit modifier 3), invulnerable from the moment it saw it until it stands
-- down. Letting go of A first ends it.
--
-- The column (0x0800FEC4, BN6's `sub_801273E`): from its side's back column forward, the first whose panel holds
-- the other side's navi on its own side's area (side 0: an other-navi panel not of side 1; side 1: one of side 1;
-- 0x0800FF08), bottom row up.
--
-- After each phase: requests 0x200 and 0x400 (a trap's counter) take over and the invulnerability ends; request
-- 0x80000 (BN5's no-charge timer's end, 0x0800DBE0) starts its action (the roles' `stun_strike`, BN5's action
-- 0x49) and the invulnerability ends.
--
-- Not modeled: at the slash, the other player's console shows Slasher's name again (sub_801EB18's counterpart,
-- presentation only).

local attachment = require("../../objects/attachment/attachment")
local regions = require("@exelib/regions")
local roles = require("../../rules/roles")
local swords = require("../../lib/swords")

local slasher = {}

export type State = {
    -- Ticks left in the slash (AIAttackVars+0x10).
    timer: number,
    -- The column it slashes (AIAttackVars+0x16; its row, +0x17, is 2).
    target_x: number,
}

slasher.STATE = { timer = "u16", target_x = "u8" }

local START, WAIT, SLASH, RECOVER = 0x0, 0x4, 0x8, 0xC
-- What the column search looks for, by side (0x0800FF08): a panel's flags required and forbidden; and where it
-- starts (0x0800FF18).
local SEARCH = {
    [0] = { require = 0x00200000, forbid = 0x00000020, from = 1 },
    [1] = { require = 0x00400020, forbid = 0x00000000, from = 6 },
}
local ROW = 2

-- 0x0800FEC4: the column, or 0.
local function find(me: Object): number
    local s = SEARCH[me.alliance]
    local dir = if me.alliance == 0 then 1 else -1
    local x = s.from
    while true do
        for y = 3, 1, -1 do
            if field.check(x, y, s.require, s.forbid) then
                return x
            end
        end
        x += dir
        if x - 1 < 0 or x - 1 > 6 then
            return 0
        end
    end
end

-- The invulnerability ends (sub_800EB08's counterpart).
local function vulnerable(me: Object)
    me:clear_invulnerable()
end

slasher.action = define.action {
    id = "slasher/action",
    state = slasher.STATE,
    update = function(me: Object, s: State)
        local step = me.step
        if step == START then
            -- 0x080ED37C: standing; at once, if the column is there.
            me:set_animation(0)
            local x = find(me)
            if x ~= 0 then
                s.target_x = x
                me.step, me.step_init = SLASH, 0
            else
                me.step, me.step_init = WAIT, 0
            end
        elseif step == WAIT then
            -- 0x080ED39E: while A is held, wait for it.
            if me:held("a") then
                local x = find(me)
                if x ~= 0 then
                    s.target_x = x
                    me:set_status_timer("invulnerable", 0xFFFF)
                    me:set_status("invulnerable", true)
                    me.step, me.step_init = SLASH, 0
                end
            else
                me:exit_attack()
            end
        elseif step == SLASH then
            -- 0x080ED3D0: the slash, on the 10th tick of 22.
            if me.step_init == 0 then
                me:set_animation(5)
                me:open_counter_window()
                battle.play_sound(swords.swing)
                attachment.attach(me, swords.blade, "related", { anim = swords.blade_anim(me) })
                me:set_status("using_action", true)
                s.timer = 0x15
                me.step_init = 4
            end
            if s.timer == 0x0C then
                local word = int.u32(me.attack_damage + me.hit_param * 0x10000 + me.extra)
                battle.hitbox(me, {
                    panel_x = s.target_x,
                    panel_y = ROW,
                    element = me.attack_element,
                    z = 0x10_0000,
                    region = regions.wide,
                    hit_effect = nil,
                    target = swords.slashes_navis.hits,
                    self_type = swords.slashes_navis.is,
                    damage = bit32.band(word, 0xFFFF),
                    stamina = bit32.rshift(word, 16),
                    hit_mod = 3,
                })
                local cx, cy = field.center(s.target_x, ROW)
                battle.effect(Vec3.new(cx, cy, 0x10_0000), swords.effects.wide, bit32.bxor(me.alliance, me.flip), 0)
            end
            local t = s.timer - 1
            s.timer = t
            if t < 0 then
                me.step, me.step_init = RECOVER, 0
            end
        elseif step == RECOVER then
            -- 0x080ED478: 6 ticks, then back to idle.
            if me.step_init == 0 then
                s.timer = 5
                me.step_init = 4
            end
            local t = s.timer - 1
            s.timer = t
            if t < 0 then
                me.related1 = nil
                me.overlay = nil
                vulnerable(me)
                me:exit_attack()
            end
        else
            error(string.format("Slasher phase %#x reads past its phase table", step))
        end
        if me:request("anti_damage_triggered") or me:request("anti_sword_triggered") then
            me:check_reactive_abort()
            vulnerable(me)
        end
        if me:request("stun_strike") then
            local a = (roles.actions :: any).stun_strike
            if a == nil then
                error("Slasher: request 0x80000 starts BN5's action 0x49 (the roles' stun_strike), which no BN5 role names yet")
            end
            me:set_attack(a, 0)
            vulnerable(me)
        end
    end,
}

return slasher
