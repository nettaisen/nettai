--!strict
-- MagnetMan's own chip, NSTackle (EXE5 chip 0x198), which his custom screen offers until it is picked (the navi's
-- `own_chip`): his own action 8 (0x080F11EE). He hits his own panel lightly (a one-tick hit of 1, hitting navis)
-- and gathers himself (animation 17, 4 ticks, marked as moving); then he is off the field, his panel reserved and
-- the chips' icons hidden, and two magnets, the MagnetMan chips' (chips/magnetmn/magnet, in their variant 1: no
-- immobilizing, a flinching hit, stopping while dimmed), go at once: his target is the first panel ahead along his
-- row that holds an enemy navi's body; one magnet (pole 1) comes from the panel as far past the target as he is
-- short of it, when that panel is on the field and free, and the other (pole 0) from his own panel, the two
-- meeting at the target. With no partner, or no target, the one from his panel slides to the field's end. When
-- that one is gone he is back on his panel (animation 3, 4 ticks) and to idle. Its damage is by his level (the
-- chips' damage formula 58: the damage rows' row 9).
--
-- He also spawns a magnet's pull (chips/magnetmn/pull) on the panel ahead of his, in the variant that follows a
-- navi through the enemy MagnetMan's attack: a player's NSTackle is another action, so the pull is on that panel
-- for the tick it appears and goes on the next.
-- Its record is written by tools/exe5/gen_content.py (bn6battle-verify) from the EXE5 ROMs.

local navi_level = require("../../lib/navi_level")
local collision = require("../../rules/collision")
local magnet = require("../../chips/magnetmn/magnet")
local marker = require("../../lib/navi_chips/marker")
local pull = require("../../chips/magnetmn/pull")
local regions = require("@exelib/regions")

export type State = {
    -- AIAttackVars +0x10: the step's ticks left.
    timer: number,
    -- AIAttackVars +0x0D and +0x0E: his target's column (0: none), and the column the second magnet comes from
    -- (0: none).
    target_x: number,
    mirror_x: number,
    -- AIAttackVars +0x30: the magnet from his own panel.
    magnet: Object?,
}

-- The steps, on the attack's step.
local GATHER, SEND, AWAY, RETURN = 0x0, 0x4, 0x8, 0xC
local ANIM_GATHER, ANIM_RETURN = 17, 3
local GATHER_TICKS, RETURN_TICKS = 4, 4
-- The magnets' variant (their fourth parameter, 1): hit modifier 3, no status; they go at once, and stop while dimmed.
local VARIANT = magnet.variant { hit_mod = 3, waits_for_navi = false, runs_while_dimmed = false }
-- A panel holding the other side's navi's body, by side (0x080F137C), and one something stands on (as words).
local ENEMY_NAVI = { [0] = 0x0400_0000, 0x0800_0000 }
local TAKEN = 0x0F80_0000
-- Where he is while away (off the field).
local AWAY_X, AWAY_Y = 10, 10

-- `object_isValidPanel`: a battle panel (columns 1 to 6, rows 1 to 3).
local function on_field(x: number, y: number): boolean
    return x >= 1 and x <= 6 and y >= 1 and y <= 3
end

-- 0x080F135A (`object_getFirstPanelInDirectionFiltered`): from (x, y) on along the row, his way, the first column
-- holding an enemy navi's body; 0 if none before the field's end.
local function find_target(me: Object, x: number, y: number): number
    local need = ENEMY_NAVI[me.alliance]
    while true do
        if field.check(x, y, need, 0) then
            return x
        end
        x += me.facing
        if not on_field(x, y) then
            return 0
        end
    end
end

local function count(s: State): boolean
    local t = s.timer - 1
    s.timer = bit32.band(t, 0xFFFF)
    return t <= 0
end

-- 0x080F1258: off the field; his target and the panel past it; the magnets and the pull.
local function send(me: Object, s: State)
    local x, y = me.panel_x, me.panel_y
    me.future_panel_x, me.future_panel_y = x, y
    me:reserve_panel(x, y)
    me.panel_x, me.panel_y = AWAY_X, AWAY_Y
    me:set_coordinates_from_panel()
    battle.show_hud({ "chip_icons" }, false)
    me.collision:set_region(nil)
    local element = me.attack_element
    local damage = int.u32(me.attack_damage + me.hit_param * 0x10000 + me.extra)
    -- (The magnets keep the navi object's own parameter word as their chip's level: a player's is 0.)
    local function magnet_at(column: number, pole: number): Object?
        return magnet.spawn(me, column, y, element, {
            pole = pole,
            paired = s.mirror_x ~= 0,
            target_x = s.target_x,
            variant = VARIANT,
            level = 0,
            damage = damage,
        })
    end
    local target = find_target(me, x, y)
    s.target_x = target
    if target ~= 0 then
        -- 0x080F1384: the panel as far past the target as he is short of it, if it is on the field and free.
        local mirror = target + (target - x)
        if not field.check(mirror, y, 0, TAKEN) then
            mirror = 0
        end
        s.mirror_x = mirror
        if mirror ~= 0 then
            magnet_at(mirror, 1)
        end
    end
    s.magnet = magnet_at(x, 0)
    pull.spawn_for_navi(me, x + me.facing, y, element, me.facing)
    me.step, me.step_init = AWAY, 0
end

local action = new.action {
    state = { timer = "u16", target_x = "u8", mirror_x = "u8", magnet = "object" },
    -- 0x080F11EE
    update = function(me: Object, s: State)
        local step = me.step
        if step == GATHER then
            -- 0x080F1210
            if me.step_init == 0 then
                me.step_init = 4
                me:set_status("moving", true)
                battle.hitbox(me, {
                    panel_x = me.panel_x,
                    panel_y = me.panel_y,
                    element = me.attack_element,
                    z = 0,
                    region = regions.single,
                    hit_effect = nil,
                    target = collision.hits_navis,
                    self_type = collision.attack,
                    damage = 1,
                    stamina = 0,
                    hit_mod = 0,
                })
                me.anim = ANIM_GATHER
                s.timer = GATHER_TICKS
            elseif count(s) then
                me.step, me.step_init = SEND, 0
            end
        elseif step == SEND then
            send(me, s)
        elseif step == AWAY then
            -- 0x080F12FC: the operation battle's mark on his side's back column each tick (0x080F13A2); until the
            -- magnet from his panel is going (one that never came keeps him away).
            marker.spawn(me, if me.facing > 0 then 1 else 6, me.future_panel_y, 1, 8, true)
            local m = s.magnet
            if m and m.lifecycle == "destroy" then
                me.step, me.step_init = RETURN, 0
            end
        elseif step == RETURN then
            -- 0x080F1314
            if me.step_init == 0 then
                me.step_init = 4
                me.collision:set_region(regions.single)
                local x, y = me.future_panel_x, me.future_panel_y
                me.panel_x, me.panel_y = x, y
                me:unreserve_panel(x, y)
                me:set_coordinates_from_panel()
                battle.show_hud({ "chip_icons" }, true)
                me:set_status("moving", false)
                me.anim = ANIM_RETURN
                s.timer = RETURN_TICKS
            elseif count(s) then
                me:exit_attack()
            end
        else
            error(string.format("MagnetMan's NSTackle's step %#x reads past its table (0x080F1200)", step))
        end
    end,
}

local chip: Chip = {
    codes = { "M" },
    element = "elec",
    family = "elec",
    class = "special",
    rarity = 0,
    mb = 99,
    damage = navi_level.row({ 80, 80, 80, 100, 100, 120, 120 }),
    hit_param = 158,
    lockout = 0,
    flags = { "has_damage", "library", "variable_damage" },
    slot_in_limit = 1,
    action = action,
}

return { nstackle = chip }
