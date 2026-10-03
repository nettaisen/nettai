--!strict
-- DblBeast's dimming controller (the original's effect object #0x7F,
-- `sub_80E9310`; docs/engine/dimming-chip-effects.md §19): the usual
-- dimming phases (@common/dimming), with an effect that warps the user out,
-- sends both beasts (./gregar, ./falzar), waits until both are done, and
-- warps the user back in.

local dimming_chips = require("@common/dimming")
local gregar = require("./gregar")
local falzar = require("./falzar")

local controller = {}

export type State = {
    -- The Atk+ / cross bonus (object +0x32): the beasts' hits add their
    -- damage to it.
    bonus: number,
    -- The beasts are out (the original's first and second parameters,
    -- whose addresses they hold).
    gregar_out: boolean,
    falzar_out: boolean,
}

-- Every wait: 31 ticks (the first sets the count).
local WAIT = 0x1E

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

-- `sub_80E93A0`: the beasts, with the bonus as their damage base (the
-- damage word's low half replaced by it); once both are done, on.
local function beasts(me: Object)
    local s = me.state :: State
    if me.phase_init == 0 then
        me.phase_init = 4
        local user = me.related1 :: Object
        local base = bit32.bor(s.bonus, bit32.lshift(me.stamina, 16))
        gregar.spawn(me, user, me.element, base)
        falzar.spawn(me, user, me.element, base)
    end
    if not s.gregar_out and not s.falzar_out then
        me.phase, me.phase_init = 8, 0
    end
end

-- `sub_80E9354`: by phase.
local function effect(me: Object)
    local phase = me.phase
    local user = me.related1 :: Object
    if phase == 0 then
        -- sub_80E9378: the user warps out; 31 ticks.
        wait(me, function()
            me.phase, me.phase_init = 4, 0
        end, function()
            navi_chip.warp(user, true)
        end)
    elseif phase == 4 then
        beasts(me)
    elseif phase == 8 then
        -- sub_80E93F2: 31 ticks.
        wait(me, function()
            me.phase, me.phase_init = 0xC, 0
        end)
    elseif phase == 0xC then
        -- sub_80E9412: the user warps back in; 31 ticks; the effect is over.
        wait(me, function()
            dimming_chips.done(me)
        end, function()
            navi_chip.warp(user, false)
        end)
    else
        error(string.format("DblBeast's controller: phase %#x reads past off_80E9368", phase))
    end
end

controller.kind = define.kind {
    id = "bn6:dblbeast/controller",
    pool = "effect",
    state = { bonus = "u16", gregar_out = "bool", falzar_out = "bool" },
    -- `sub_80E9310`.
    update = dimming_chips.phases { name = "DblBeast", effect = effect },
}

-- `sub_80E943E`: DblBeast's `dimming` hook.
function controller.hook(user: Object, spec: DimmingChipSpec): Object?
    local o = dimming_chips.spawn(controller.kind, user, spec)
    if o then
        (o.state :: State).bonus = spec.bonus
    end
    return o
end

return controller
