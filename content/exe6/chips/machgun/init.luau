--!strict
-- MachGun1-3: nine panel strikes (objects/panel_strike) on a target that
-- sweeps up and down the first column ahead with an enemy in it, a strike
-- every nine ticks, each marked by a reticle (the original's action 0x29,
-- `sub_80ECF2E`; docs/engine/shot-chips.md §8). When the sweep turns at
-- the field's edge, the target moves a column toward the enemy's if the
-- enemy left it. The three differ only in their damage.

local attachment = require("../objects/attachment")
local panel_strike = require("../objects/panel_strike")
local panels = require("@exelib/panels")

export type State = {
    -- The countdown to the next strike (AIAttackVars+0x10).
    timer: number,
    -- Strikes left (AIAttackVars+0x0E).
    strikes: number,
    -- The target (AIAttackVars+0x16, +0x17) and the way it sweeps the
    -- column (+0x30: 1 down, -1 up).
    column: number,
    row: number,
    sweep: number,
}

local STATE = { timer = "u16", strikes = "u8", column = "u8", row = "u8", sweep = "i8" }

-- The phases, on the attack's step.
local AIM, FIRE = 0x0, 0x4
-- FIRE's step_init: the first strike's entry, a strike, the countdown.
local FIRST, STRIKE, COUNTDOWN = 0, 2, 1

local ANIM_HOLD, ANIM_FIRE = 0xA, 0xD
-- Strikes, the ticks between them (less one), and the row it starts on.
local STRIKES, INTERVAL, START_ROW = 9, 8, 3
-- The panel strike: no crack, after 6 ticks, hit modifier 1 (the chips'
-- parameters, the third forced to 1).
local PANEL_STRIKE: PanelStrike = { warning = 6, hit_mod = 1 }
-- The reticle (effect 0x2F) over the target, for 6 ticks (the chips'
-- second parameter).
local RETICLE = new.effect { sprite = asset.sprite("reticle"), anim = 0, palette = 0 }
local RETICLE_TICKS = 6
-- The gun's animation as it fires.
local GUN_FIRE = 1

-- The machine gun (the original's attachment row 0x1D).
local GUN = attachment.look { sprite = asset.sprite("machine-gun"), palette = 0, attach_point = 26 }

-- `sub_80ED040` (@exelib/panels, the dragons' too): from the column ahead
-- forward, the first with an enemy in some row; none, the last column
-- ahead.
local function target_column(me: Object): number
    local x = panels.enemy_column(me)
    return x
end

-- `sub_80ECF48`: the Atk+ bonus goes into the damage word; aim at the
-- bottom of the target column; the gun.
local function aim(me: Object, s: State)
    local word = int.u32(me.attack_damage + me.hit_param * 0x10000 + me.extra)
    me.attack_damage = bit32.band(word, 0xFFFF)
    me.hit_param = bit32.rshift(word, 16)
    s.column = target_column(me)
    s.row = START_ROW
    me.anim = ANIM_HOLD
    me:open_counter_window()
    attachment.attach(me, GUN, "overlay")
    me:set_status("using_action", true)
    s.strikes = STRIKES
    s.sweep = -1
    me.step, me.step_init = FIRE, 0
end

-- `sub_80ECF8E`: a strike on the target, with the reticle; nine ticks
-- later the target moves on, and the next strike, or the end.
local function fire(me: Object, s: State)
    local init = me.step_init
    if init ~= COUNTDOWN then
        if init == FIRST then
            local gun = me.overlay
            if gun then
                gun.anim = GUN_FIRE
            end
            me:set_animation(ANIM_FIRE)
        end
        me.step_init = COUNTDOWN
        s.timer = INTERVAL
        local damage = int.u32(me.attack_damage + me.hit_param * 0x10000)
        panel_strike.spawn(me, s.column, s.row, me.attack_element, PANEL_STRIKE, damage, 0, false)
        local cx, cy = field.center(s.column, s.row)
        local reticle = battle.effect(Vec3.new(cx, cy, 0), RETICLE)
        if reticle then
            reticle.timer = RETICLE_TICKS
        end
    end
    local t = s.timer - 1
    s.timer = t
    if t >= 0 then
        return
    end
    local row = s.row + s.sweep
    if row >= 1 and row <= 3 then
        s.row = row
    else
        s.sweep = -s.sweep
        local target = target_column(me)
        if target == s.column then
            s.row += s.sweep
        elseif target > s.column then
            s.column += 1
        else
            s.column -= 1
        end
    end
    local left = s.strikes - 1
    s.strikes = left
    if left ~= 0 then
        me.step_init = STRIKE
        return
    end
    me.related1 = nil
    me.overlay = nil
    me:exit_attack()
end

-- A chip's action (the three are alike; each chip has its own).
local function mach_gun(): Action
    return new.action {
        state = STATE,
        -- `sub_80ECF2E`.
        update = function(me: Object, s: State)
            local step = me.step
            if step == AIM then
                aim(me, s)
            elseif step == FIRE then
                fire(me, s)
            else
                error(string.format("MachGun phase %#x reads past off_80ECF40", step))
            end
        end,
    }
end

local chips: { [string]: Chip } = {
    machgun1 = {
        codes = { "A", "R", "T", "*" },
        element = "null", family = "cursor", class = "standard",
        rarity = 0, mb = 12, damage = 30, hit_param = 30, lockout = 0,
        flags = { "has_damage", "standard_library", "library" },
        slot_in_limit = 3,
        action = mach_gun(),
    },
    machgun2 = {
        codes = { "E", "G", "S" },
        element = "null", family = "cursor", class = "standard",
        rarity = 1, mb = 24, damage = 50, hit_param = 30, lockout = 0,
        flags = { "has_damage", "standard_library", "library" },
        slot_in_limit = 3,
        action = mach_gun(),
    },
    machgun3 = {
        codes = { "B", "F", "M" },
        element = "null", family = "cursor", class = "standard",
        rarity = 2, mb = 36, damage = 70, hit_param = 30, lockout = 0,
        flags = { "has_damage", "standard_library", "library" },
        slot_in_limit = 3,
        action = mach_gun(),
    },
}

return chips
