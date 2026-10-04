--!strict
-- MetaGel's dimming controller (BN5's effect object #0x21, 0x080E2D98; the dimming handler's subtype 6, 0x080E2E9C):
-- the usual dimming phases (@exelib/dimming), with an effect (0x080E2DDC) that drops a gel (./gel) on each row's
-- front panel of the other side's area, one every 13 ticks in a shuffled order of the rows, then waits until they
-- are all gone.

local dimming_chips = require("@exelib/dimming")
local gel = require("./gel")
local panels = require("@exelib/panels")

local controller = {}

export type State = {
    -- The Atk+ bonus (object +0x32): it adds to the gels' damage word.
    bonus: number,
    -- The rows left (ExtraVars), their order (ExtraVars+4), and the gels still out (ExtraVars+8, a byte each).
    left: number,
    order: { number },
    out: { boolean },
}

local GEL_TICKS = 12

-- 0x080E2DDC, by phase.
local function effect(me: Object)
    local s = me.state :: State
    local phase = me.phase
    if phase == 0 then
        -- 0x080E2DFC: three rows, shuffled (four swaps).
        s.left = 3
        for k = 1, 4 do
            s.out[k] = false
        end
        local order = { 1, 2, 3 }
        panels.shuffle(order, 3, 4)
        for k = 1, 3 do
            s.order[k] = order[k]
        end
        me.timer = GEL_TICKS
        me.phase, me.phase_init = 4, 0
    elseif phase == 4 then
        -- 0x080E2E24: a gel every 13 ticks on the next row's front panel (none: a row passed).
        local t = me.timer - 1
        me.timer = bit32.band(t, 0xFFFF)
        if t >= 0 then
            return
        end
        s.left -= 1
        if s.left < 0 then
            me.phase, me.phase_init = 8, 0
            return
        end
        local y = s.order[s.left + 1]
        local x = panels.area_front(me.alliance, me.flip, y)
        if x ~= 0 then
            -- (It runs while the battle is dimmed.)
            local g = gel.spawn(me, x, y, dimming_chips.damage_plus(me, s.bonus), s.left + 1)
            if g then
                g.run_while_dimmed = true
            end
        end
        me.timer = GEL_TICKS
    elseif phase == 8 then
        -- 0x080E2E8C: once every gel is gone.
        for k = 1, 4 do
            if s.out[k] then
                return
            end
        end
        dimming_chips.done(me)
    else
        error(string.format("MetaGel's controller: phase %#x reads past its table", phase))
    end
end

controller.kind = define.kind {
    id = "metagel/controller",
    pool = "effect",
    state = { bonus = "u16", left = "i32", order = "u8[3]", out = "bool[4]" },
    update = dimming_chips.phases { name = "MetaGel", effect = effect },
}

-- 0x080E2E9C: MetaGel's `dimming` hook.
function controller.hook(user: Object, spec: DimmingChipSpec): Object?
    local o = dimming_chips.spawn(controller.kind, user, spec)
    if o then
        (o.state :: State).bonus = spec.bonus
    end
    return o
end

return controller
