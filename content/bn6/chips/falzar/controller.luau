--!strict
-- Falzar's dimming controller (the original's effect object #0x7B; the
-- Japanese ROMs' 0x080EDE8C in the Falzar ROM, 0x080EF1BC in the Gregar
-- ROM; the US ROMs point the slot at a placeholder and their `off_802CCB4`
-- entry 35 is null: docs/engine/beast-chips.md). The dimming's phases
-- (@common/dimming) with its own first and last: the field fades to black
-- rather than dims, and back from black. Its effect hides the gauge and
-- the emotion window and warps the user out; the field's obstacles break;
-- for 120 ticks a feather dives every 12 ticks onto a panel of the other
-- side's (./feather); then Falzar comes (three pieces of ./summon over the
-- user), and once it is done the user and the HUD are back.

local dimming_chips = require("@common/dimming")
local feather = require("./feather")
local summon = require("./summon")
local targets = require("../../lib/beast-chips/targets")

local controller = {}

export type State = {
    -- The Atk+ / cross bonus (+0x32).
    bonus: number,
    -- Falzar is out (its Param1, whose address the summon holds).
    falzar_out: boolean,
    -- The last panel a feather was aimed at (+0x60, +0x64).
    last_x: number,
    last_y: number,
}

-- Every wait: 31 ticks (the first sets the count).
local WAIT = 0x1E
-- The feathers: for 120 ticks, one every 12; acting while dimmed (their
-- Param2; their Param1, 3, picks the same speed as any other).
local FEATHER_TICKS, FEATHER_EVERY = 0x78, 0x0C
-- The pieces of Falzar: (leads, first animation), in the order they come.
type Piece = { leads: boolean, anims: number }
local PIECES: { Piece } = {
    { leads = true, anims = 0 },
    { leads = false, anims = 4 },
    { leads = false, anims = 8 },
}

-- 0x080EE1EC: the custom gauge and the emotion window hide (`shown`
-- false: `sub_801DACC`) or show (`sub_801DA48`): HUD draw tasks 4 and 14,
-- or with each side's own gauge (battle flag 0x40) 17 and 14.
local function hud(shown: boolean)
    local parts: { HudPart } = if battle.per_player_gauges()
        then { "level_gauge", "emotion_window" }
        else { "gauge", "emotion_window" }
    battle.show_hud(parts, shown)
end

-- 0x080EE21C: every field object's HP to 0 (the registry's eight slots,
-- stage objects included), which breaks them.
local function clear_field()
    for _, o in field.all_objects() do
        o.hp = 0
    end
end

local function to_phase(me: Object, phase: number)
    me.phase, me.phase_init = phase, 0
end

-- A wait of the effect's phases: the first tick sets the count (after
-- `first`), then the count runs down to `next`.
local function wait(me: Object, next: () -> (), first: (() -> ())?)
    if me.phase_init == 0 then
        me.phase_init = 4
        if first then
            first()
        end
        me.timer = WAIT
        return
    end
    local t = me.timer - 1
    me.timer = t
    if t <= 0 then
        next()
    end
end

-- 0x080EE180: a feather onto (x, y), with the controller's damage word
-- (no bonus). (The spawner's Z register holds 0x100, the word's Param2
-- byte as it was built.)
local function send_feather(me: Object, x: number, y: number)
    local word = bit32.bor(me.damage, bit32.lshift(me.stamina, 16))
    feather.spawn(me, x, y, me.element, word, true, 0x100)
end

-- 0x080EDF52: 120 ticks, from the first a feather every 12 on a fresh
-- panel.
local function feathers(me: Object)
    local s = me.state :: State
    if me.phase_init == 0 then
        me.phase_init = 4
        s.last_x, s.last_y = 0, 0
        me.timer = FEATHER_TICKS
        me.timer2 = 0
    end
    local t2 = me.timer2 - 1
    me.timer2 = t2
    if t2 <= 0 then
        me.timer2 = FEATHER_EVERY
        local x, y = targets.pick(me, s.last_x, s.last_y)
        if x and y then
            s.last_x, s.last_y = x, y
            send_feather(me, x, y)
        end
    end
    local t = me.timer - 1
    me.timer = t
    if t <= 0 then
        to_phase(me, 8)
    end
end

-- 0x080EDFB6: Falzar's three pieces over the user, with the controller's
-- damage word plus the bonus; once it is done, on. (Each spawn's Z
-- register holds the piece's animation byte as the parameter word was
-- built.)
local function send(me: Object)
    local s = me.state :: State
    if me.phase_init == 0 then
        me.phase_init = 4
        local user = me.related1 :: Object
        local word = int.u32(bit32.bor(me.damage, bit32.lshift(me.stamina, 16)) + s.bonus)
        for _, p in PIECES do
            summon.spawn(me, user, me.panel_x, me.panel_y, me.element, word, p.leads, p.anims, p.anims * 0x100)
        end
    end
    if not s.falzar_out then
        to_phase(me, 0x10)
    end
end

-- 0x080EDEF6: the effect, by phase (0x080EDF08).
local function effect(me: Object)
    local phase = me.phase
    local user = me.related1 :: Object
    if phase == 0 then
        -- 0x080EDF20: the HUD hides, the user warps out; 31 ticks; the
        -- field's obstacles break.
        wait(me, function()
            clear_field()
            to_phase(me, 4)
        end, function()
            hud(false)
            navi_chip.warp(user, true)
        end)
    elseif phase == 4 then
        feathers(me)
    elseif phase == 8 then
        -- 0x080EDF96: 31 ticks.
        wait(me, function()
            to_phase(me, 0xC)
        end)
    elseif phase == 0xC then
        send(me)
    elseif phase == 0x10 then
        -- 0x080EE034: 31 ticks.
        wait(me, function()
            to_phase(me, 0x14)
        end)
    elseif phase == 0x14 then
        -- 0x080EE054: the user warps back in; 31 ticks; the HUD is back and
        -- the effect is over.
        wait(me, function()
            hud(true)
            dimming_chips.done(me)
        end, function()
            navi_chip.warp(user, false)
        end)
    else
        error(string.format("Falzar's controller: phase %#x reads past its table (0x080EDF08)", phase))
    end
end

controller.kind = define.kind {
    id = "bn6:falzar/controller",
    pool = "effect",
    state = { bonus = "u16", falzar_out = "bool", last_x = "u32", last_y = "u32" },
    -- 0x080EDE8C: the dimming's phases (0x080EDEAC, by action:
    -- 0x080EDED0, `object_drawChipName`, the effect, `sub_800BCF6`).
    update = dimming_chips.phases {
        effect = effect,
        dim = dimming.fade_to_black,
        undim = dimming.fade_from_black,
    },
}

-- 0x080EE086: Falzar's `dimming` hook.
function controller.hook(user: Object, spec: DimmingChipSpec): Object?
    local o = dimming_chips.spawn(controller.kind, user, spec)
    if o then
        (o.state :: State).bonus = spec.bonus
    end
    return o
end

return controller
