--!strict
-- Meteors' dimming controller (the original's effect object #0x34,
-- `sub_80E4188`; docs/engine/dimming-chip-effects.md §6): thirty meteors
-- (./falling_meteor, row 1) fall on the enemy's area, one every 10 ticks,
-- cycling through its panels from the far column. Another game's
-- controller drops its own meteors (`controller.make`; BN5's Meteors).

local dimming_chips = require("@common/dimming")
local falling_meteor = require("./falling_meteor")

export type State = {
    -- The Atk+ / cross bonus the telop shows (+0x32), which the meteors'
    -- damage adds.
    bonus: number,
    -- The target panels (x | y << 4, from the original's +0x60), how many,
    -- and the next one.
    panels: { number },
    count: number,
    next: number,
}

local FIRST, INTERVAL, METEORS, AFTER = 2, 0xA, 0x1E, 0x5A

-- `sub_800D5F0`: how many panels of column `x` aren't `side`'s.
local function foreign_panels(x: number, side: number): number
    local n = 0
    for y = 1, 3 do
        local p = field.panel(x, y)
        if p and p.alliance ~= side then
            n += 1
        end
    end
    return n
end

-- `sub_80E42AA`: the other side's panels, column by column from the far
-- column back, from the first column holding one to the first holding
-- none.
local function targets(me: Object, s: State): number
    local step = -me.facing
    local x = 6 - 5 * bit32.bxor(me.alliance, me.flip)
    while foreign_panels(x, me.alliance) == 0 do
        x += step
        if x < 1 or x > 6 then
            return 0
        end
    end
    local n = 0
    while true do
        local found = 0
        for y = 1, 3 do
            local p = field.panel(x, y)
            if p and p.alliance ~= me.alliance then
                n += 1
                s.panels[n] = bit32.bor(x, bit32.lshift(y, 4))
                found += 1
            end
        end
        if found == 0 then
            return n
        end
        x += step
        if x < 1 or x > 6 then
            return n
        end
    end
end

local controller = {}

-- A game's controller (kind `id`) dropping meteors of `meteor_kind`: its
-- chip's `dimming` hook.
function controller.make(id: string, meteor_kind: Kind): (user: Object, spec: DimmingChipSpec) -> Object?
    -- `sub_80E41CC`, by phase: list the targets (none: the effect ends); a
    -- meteor every 10 ticks, the first 2 ticks on, thirty; 91 ticks.
    local function effect(me: Object)
        local s = me.state :: State
        local phase = me.phase
        if phase == 0 then
            s.count = targets(me, s)
            if s.count == 0 then
                dimming_chips.done(me)
                return
            end
            me.timer, me.timer2 = FIRST, METEORS
            s.next = 0
            me.phase, me.phase_init = 4, 0
        elseif phase == 4 then
            local t = me.timer - 1
            me.timer = t
            if t ~= 0 then
                return
            end
            me.timer = INTERVAL
            local left = me.timer2 - 1
            me.timer2 = left
            if left < 0 then
                me.phase, me.phase_init = 0xC, 0
                return
            end
            local i = s.next
            s.next = if i + 1 < s.count then i + 1 else 0
            local b = s.panels[i + 1]
            local x, y = bit32.band(b, 7), bit32.rshift(b, 4)
            local damage = dimming_chips.damage_plus(me, s.bonus)
            falling_meteor.spawn_kind(meteor_kind, 1, me, x, y, me.element, damage, true, Vec3.new(y, me.element, s.bonus))
        elseif phase == 8 then
            -- (Nothing reaches it: the table's third entry.)
            dimming.dim_screen(me)
        elseif phase == 0xC then
            if me.phase_init == 0 then
                me.timer = AFTER
                me.phase_init = 4
            end
            local t = me.timer - 1
            me.timer = t
            if t < 0 then
                dimming_chips.done(me)
            end
        else
            error(string.format("Meteors' effect: phase %#x reads past off_80E41E0", phase))
        end
    end

    local kind = define.kind {
        id = id,
        pool = "effect",
        state = { bonus = "u16", panels = "u8[20]", count = "u32", next = "u32" },
        -- `sub_80E4188`.
        update = dimming_chips.phases { name = "Meteors", effect = effect },
    }

    -- The chip's `dimming` hook (`sub_80E4288`).
    return function(user: Object, spec: DimmingChipSpec): Object?
        local o = dimming_chips.spawn(kind, user, spec)
        if o then
            (o.state :: State).bonus = spec.bonus
        end
        return o
    end
end

-- BN6's.
controller.hook = controller.make("bn6:meteors/controller", falling_meteor.kind)

return controller
