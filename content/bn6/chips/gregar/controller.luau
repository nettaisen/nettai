--!strict
-- Gregar's dimming controller (the original's effect object #0x7A; the
-- Japanese ROMs' 0x080EDCC8 in the Falzar ROM, 0x080EEFF8 in the Gregar
-- ROM; the US ROMs point the slot at a placeholder and their `off_802CCB4`
-- entry 34 is null: docs/engine/beast-chips.md). The dimming's phases
-- (lib/dimming) with its own first and last: the field fades to black
-- rather than dims, and back from black. Its effect hides the gauge and
-- the emotion window, warps the user out, sends Gregar (two pieces of
-- ./summon, one panel behind the user), waits for it to be done, and
-- brings the user and the HUD back.

local dimming_chips = require("../../lib/dimming")
local summon = require("./summon")

local controller = {}

export type State = {
    -- The Atk+ / cross bonus (+0x32).
    bonus: number,
    -- The rocks' damage (its Param1: the chip's parameter byte).
    rock_damage: number,
    -- Gregar is out (its Param3, whose address the summon holds).
    gregar_out: boolean,
}

-- What its use takes: the rocks' damage (the chip's parameter byte).
export type Spec = { rock_damage: number }

-- Every wait: 31 ticks (the first sets the count).
local WAIT = 0x1E
-- The pieces of Gregar.
local PIECES = 2

-- 0x080EDE5E: the custom gauge and the emotion window hide (`shown`
-- false: `sub_801DACC`) or show (`sub_801DA48`): HUD draw tasks 4 and 14,
-- or with each side's own gauge (battle flag 0x40) 17 and 14.
local function hud(shown: boolean)
    local parts: { HudPart } = if battle.per_player_gauges()
        then { "level_gauge", "emotion_window" }
        else { "gauge", "emotion_window" }
    battle.show_hud(parts, shown)
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

local function to_phase(me: Object, phase: number)
    me.phase, me.phase_init = phase, 0
end

-- 0x080EDD82: Gregar's two pieces one panel behind the user, with the
-- controller's damage word, bonus and rocks' damage; once it is done, on.
local function send(me: Object)
    local s = me.state :: State
    if me.phase_init == 0 then
        me.phase_init = 4
        local user = me.related1 :: Object
        local word = bit32.bor(me.damage, bit32.lshift(me.stamina, 16))
        for piece = 0, PIECES - 1 do
            summon.spawn(me, user, me.panel_x - me.facing, me.panel_y, me.element, word, s.bonus, piece, s.rock_damage)
        end
    end
    if not s.gregar_out then
        to_phase(me, 8)
    end
end

-- 0x080EDD32: the effect, by phase (0x080EDD44).
local function effect(me: Object)
    local phase = me.phase
    local user = me.related1 :: Object
    if phase == 0 then
        -- 0x080EDD54: the HUD hides, the user warps out; 31 ticks.
        wait(me, function()
            to_phase(me, 4)
        end, function()
            hud(false)
            navi_chip.warp(user, true)
        end)
    elseif phase == 4 then
        send(me)
    elseif phase == 8 then
        -- 0x080EDDEA: 31 ticks.
        wait(me, function()
            to_phase(me, 0xC)
        end)
    elseif phase == 0xC then
        -- 0x080EDE0A: the user warps back in; 31 ticks; the HUD is back and
        -- the effect is over.
        wait(me, function()
            hud(true)
            dimming_chips.done(me)
        end, function()
            navi_chip.warp(user, false)
        end)
    else
        error(string.format("Gregar's controller: phase %#x reads past its table (0x080EDD44)", phase))
    end
end

controller.kind = define.kind {
    id = "gregar/controller",
    pool = "effect",
    state = { bonus = "u16", rock_damage = "u8", gregar_out = "bool" },
    -- 0x080EDCC8: the dimming's phases (0x080EDCE8, by action:
    -- 0x080EDD0C, `object_drawChipName`, the effect, `sub_800BCF6`).
    update = dimming_chips.phases {
        effect = effect,
        dim = dimming.fade_to_black,
        undim = dimming.fade_from_black,
    },
}

-- 0x080EDE3C: Gregar's `dimming` hook, with the rocks' damage of `spec`.
function controller.hook(spec: Spec): (user: Object, chip: DimmingChipSpec) -> Object?
    local rock_damage = spec.rock_damage
    return function(user: Object, chip: DimmingChipSpec): Object?
        local o = dimming_chips.spawn(controller.kind, user, chip)
        if o then
            local s = o.state :: State
            s.bonus, s.rock_damage = chip.bonus, rock_damage
        end
        return o
    end
end

return controller
