--!strict
-- MetrKnuk's dimming controller (the original's effect object #0x76,
-- `sub_80E8B00`; docs/engine/dimming-chip-effects.md §18): the usual
-- dimming phases (lib/dimming), with an effect that throws 16 fists
-- (./fist) at the enemy's area, 8 ticks apart: the first at a random
-- panel an enemy stands on, the others at random panels around the
-- enemies (never the last one twice in a row).

local dimming_chips = require("../../lib/dimming")
local panels = require("../../lib/panels")
local fist = require("./fist")

local controller = {}

export type State = {
    -- The Atk+ / cross bonus (object +0x32): it adds to the fists' damage
    -- word.
    bonus: number,
    -- The last target (ExtraVars+0, y << 4 | x; 0: none).
    last: number,
    -- The panels around the enemies (`grid[6 * y + x + 1]`: the object's
    -- +0x64, 0x28 bytes past its variables).
    grid: { number },
}

local PUNCHES = 16
-- The ticks to the next punch, less one.
local BETWEEN = 6
-- The height the fists fall from.
local HEIGHT = 0x78_0000
-- The panels of the other side, by the controller's side, with an enemy
-- body on them (`byte_80E8D6C`), or any (`byte_80E8D58`).
local ENEMY_BODY = { [0] = { require = 0x04010020, forbid = 0 }, [1] = { require = 0x08010000, forbid = 0x20 } }
local ENEMY_AREA = { [0] = { require = 0x00010020, forbid = 0 }, [1] = { require = 0x00010000, forbid = 0x20 } }
-- `byte_80E8D80`: a body's 3x3, which `sub_80E8CC4` walks from the last.
local AROUND = { { -1, -1 }, { 0, -1 }, { 1, -1 }, { -1, 0 }, { 0, 0 }, { 1, 0 }, { -1, 1 }, { 0, 1 }, { 1, 1 } }
local GRID = 0x28

local function grid_index(x: number, y: number): number
    return 6 * y + x + 1
end

local function clear_grid(s: State)
    for i = 1, GRID do
        s.grid[i] = 0
    end
end

-- `sub_80E8C44`: a random panel an enemy stands on, or of the enemy's
-- area when none does (a `rng_positive` draw); 0 when there is no panel.
local function first_target(me: Object): number
    local side = me.alliance
    local list = panels.filtered(ENEMY_BODY[side].require, ENEMY_BODY[side].forbid)
    if #list == 0 then
        list = panels.filtered(ENEMY_AREA[side].require, ENEMY_AREA[side].forbid)
    end
    if #list == 0 then
        -- The original keeps a register the panel search left as the row.
        return 0
    end
    return list[battle.rng_positive() % #list + 1]
end

-- `sub_80E8C84`: the grid marks the 3x3 around each panel an enemy stands
-- on (inside the field); whether there was one.
local function mark_around_bodies(me: Object, s: State): boolean
    clear_grid(s)
    local cond = ENEMY_BODY[me.alliance]
    local list = panels.filtered(cond.require, cond.forbid)
    for _, p in list do
        local px, py = panels.x(p), panels.y(p)
        for i = #AROUND, 1, -1 do
            local x, y = px + AROUND[i][1], py + AROUND[i][2]
            if x >= 1 and x <= 6 and y >= 1 and y <= 3 then
                s.grid[grid_index(x, y)] = 1
            end
        end
    end
    return #list > 0
end

-- `sub_80E8CFA`: the grid marks every panel of the enemy's area.
local function mark_area(me: Object, s: State)
    clear_grid(s)
    local cond = ENEMY_AREA[me.alliance]
    for _, p in panels.filtered(cond.require, cond.forbid) do
        s.grid[grid_index(panels.x(p), panels.y(p))] = 1
    end
end

-- `sub_80E8BF8`: the marked panels of the enemy's area but the last
-- target (`sub_80E8D92`: rows 4 to 1, columns 7 to 1, past the field
-- too, whose flags meet nothing), one at random (a `rng_positive` draw);
-- 0 when none.
local function next_target(me: Object, s: State): number
    if not mark_around_bodies(me, s) then
        mark_area(me, s)
    end
    if s.last ~= 0 then
        s.grid[grid_index(bit32.band(s.last, 7), bit32.rshift(s.last, 4))] = 0
    end
    local cond = ENEMY_AREA[me.alliance]
    local list: { number } = {}
    for y = 4, 1, -1 do
        for x = 7, 1, -1 do
            local f = field.flags(x, y)
            if bit32.band(f, cond.forbid) == 0
                and bit32.band(f, cond.require) == cond.require
                and s.grid[grid_index(x, y)] ~= 0
            then
                list[#list + 1] = panels.pack(x, y)
            end
        end
    end
    if #list == 0 then
        return 0
    end
    return list[battle.rng_positive() % #list + 1]
end

-- `sub_80E8B70`: a fist at the target (the first from the bodies, then
-- around them); after the 16th the effect is over, else 7 ticks to the
-- next.
local function punch(me: Object)
    local s = me.state :: State
    local target = if me.timer2 == PUNCHES then first_target(me) else next_target(me, s)
    s.last = target
    local x = panels.x(target)
    if x ~= 0 then
        -- sub_80E8BE2 (the punch's parity, Timer2 & 1, the fist's first
        -- parameter, is read by nothing).
        fist.spawn(me, x, panels.y(target), me.element, HEIGHT, dimming_chips.damage_plus(me, s.bonus))
    end
    local n = me.timer2 - 1
    me.timer2 = n
    if n <= 0 then
        dimming_chips.done(me)
        return
    end
    me.timer = BETWEEN
    me.phase, me.phase_init = 8, 0
end

-- `sub_80E8BB0`: wait (the first time, what the dim left on the timer).
local function wait(me: Object)
    local t = me.timer - 1
    me.timer = t
    if t < 0 then
        me.phase, me.phase_init = 4, 0
    end
end

-- `sub_80E8B44`: by phase: 0 the punches' count, 4 a punch, 8 the wait.
local function effect(me: Object)
    local phase = me.phase
    if phase == 0 then
        -- sub_80E8B64.
        me.timer2 = PUNCHES
        me.phase, me.phase_init = 8, 0
    elseif phase == 4 then
        punch(me)
    elseif phase == 8 then
        wait(me)
    else
        error(string.format("MetrKnuk's controller: phase %#x reads past off_80E8B58", phase))
    end
end

controller.kind = define.kind {
    id = "metrknuk/controller",
    pool = "effect",
    state = { bonus = "u16", last = "u32", grid = "u8[40]" },
    -- `sub_80E8B00`.
    update = dimming_chips.phases { name = "MetrKnuk", effect = effect },
}

-- `sub_80E8BC0`: MetrKnuk's `dimming` hook.
function controller.hook(user: Object, spec: DimmingChipSpec): Object?
    local o = dimming_chips.spawn(controller.kind, user, spec)
    if o then
        (o.state :: State).bonus = spec.bonus
    end
    return o
end

return controller
