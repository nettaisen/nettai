--!strict
-- The meteor shower's dimming controller (effect object #0x34 in all three games: EXE6's `sub_80E4188`, its Meteors'
-- (docs/engine/dimming-chip-effects.md §6); EXE5's 0x080E41B8, its Meteors'; EXE4's 0x080E5C90, its ShotStar's): thirty
-- meteors (./falling_meteor, row 1) fall on the enemy's area, one every 10 ticks, going through its panels from the
-- far column. A game makes its own (`controller.make`) with its meteor and how it finds each meteor's panel
-- (`targets`): EXE6's and EXE5's list the enemy's panels as the effect starts and go round the list
-- (`controller.listed`); EXE4's looks for the next one as each meteor falls (`controller.scanned`).

local dimming_chips = require("../dimming")
local falling_meteor = require("./falling_meteor")

local controller = {}

export type State = {
    -- The Atk+ / cross bonus the telop shows (+0x32), which the meteors'
    -- damage adds.
    bonus: number,
}

-- How a controller finds its meteors' panels.
export type Targets = {
    -- What it keeps in the controller's state.
    state: StateSchema,
    -- As the effect starts: whether a first meteor has a panel (none: the effect ends).
    first: (me: Object) -> boolean,
    -- The panel the next meteor falls on; on to the one after.
    take: (me: Object) -> (number, number),
}

export type Look = {
    -- The kind's id.
    id: string,
    -- Its meteors' kind and row (the original's row 1).
    meteor: Kind,
    row: MeteorRow,
    targets: Targets,
}

local FIRST, INTERVAL, METEORS, AFTER = 2, 0xA, 0x1E, 0x5A

-- EXE6's and EXE5's targets: the enemy's panels listed as the effect starts.
type Listed = {
    -- The target panels (x | y << 4, from the original's +0x60), how many,
    -- and the next one.
    panels: { number },
    count: number,
    next: number,
}

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
local function list(me: Object, s: Listed): number
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

controller.listed = {
    state = { panels = "u8[20]", count = "u32", next = "u32" },
    first = function(me: Object): boolean
        local s = me.state :: Listed
        s.count = list(me, s)
        s.next = 0
        return s.count ~= 0
    end,
    take = function(me: Object): (number, number)
        local s = me.state :: Listed
        local i = s.next
        s.next = if i + 1 < s.count then i + 1 else 0
        local b = s.panels[i + 1]
        return bit32.band(b, 7), bit32.rshift(b, 4)
    end,
} :: Targets

-- EXE4's targets: the panel the next meteor falls on (+0x60, +0x64), found as the last one falls.
type Scanned = { x: number, y: number }

-- 0x080E5D9A: from the panel after the last, down its column and then column by column toward the user's side, the
-- next panel of the other side's (0x080E5E10's flags: the owner bit 0x20 set for side 0's meteors, clear for side
-- 1's); off the field, the same from the far column's top (0x080E5E24: column 6 for side 0, 1 for side 1). None: the
-- last one stays.
local function scan(me: Object, s: Scanned): boolean
    local side = me.alliance
    local step = 2 * side - 1
    local require, forbid = if side == 0 then 0x20 else 0, if side == 0 then 0 else 0x20
    local x, y = s.x, s.y + 1
    for pass = 1, 2 do
        while true do
            if y > 3 then
                y = 1
                x += step
                if x < 1 or x > 6 then
                    break
                end
            elseif field.check(x, y, require, forbid) then
                s.x, s.y = x, y
                return true
            else
                y += 1
            end
        end
        if pass == 1 then
            x = 6 - 5 * side
        end
    end
    return false
end

controller.scanned = {
    state = { x = "i32", y = "i32" },
    -- 0x080E5CF8: from before the field, past a column's last row.
    first = function(me: Object): boolean
        local s = me.state :: Scanned
        s.x, s.y = 0, 3
        return scan(me, s)
    end,
    -- 0x080E5D38: the panel found, then the next looked for.
    take = function(me: Object): (number, number)
        local s = me.state :: Scanned
        local x, y = s.x, s.y
        scan(me, s)
        return x, y
    end,
} :: Targets

-- A game's controller: its chip's `dimming` hook.
function controller.make(look: Look): (user: Object, spec: DimmingChipSpec) -> Object?
    local targets = look.targets
    -- `sub_80E41CC`, by phase: the first target (none: the effect ends); a
    -- meteor every 10 ticks, the first 2 ticks on, thirty; 91 ticks.
    local function effect(me: Object)
        local s = me.state :: State
        local phase = me.phase
        if phase == 0 then
            if not targets.first(me) then
                dimming_chips.done(me)
                return
            end
            me.timer, me.timer2 = FIRST, METEORS
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
            local x, y = targets.take(me)
            local damage = dimming_chips.damage_plus(me, s.bonus)
            falling_meteor.spawn_kind(look.meteor, look.row, me, x, y, me.element, damage, true, Vec3.new(y, me.element, s.bonus))
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

    local state: StateSchema = { bonus = "u16" }
    for name, field_type in targets.state do
        state[name] = field_type
    end

    local kind = new.kind {
        id = look.id,
        pool = "effect",
        state = state,
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

return controller
