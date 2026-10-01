--!strict
-- Snake's nest (the original's attack object #0x72, `sub_80D2BDC`;
-- docs/engine/dimming-chip-effects.md §13): it scans the user's side's
-- holes from the far column back, sending a snake (./snake) out of each
-- at the nearest enemy, 8 ticks apart in a column and 24 between columns;
-- once the snakes are gone, it lets its holder go on. It has no sprite.

local snake = require("./snake")

local nest = {}

export type State = {
    -- The scan's place (Param3, Param4: bytes, the column stepping off the
    -- field when the scan ends).
    scan_x: number,
    scan_y: number,
    -- The snakes out (ExtraVars+4).
    snakes: number,
    -- The object whose `busy` it clears when it goes (the original's
    -- pointer to its controller's second parameter).
    holder: Object?,
}

-- Its states (PhaseInitialized): starting, scanning, waiting for the
-- snakes.
local START, SCANNING, WAITING = 0, 1, 2
-- A hole on each side's area (`byte_80D2D30`): the side's own panel, not
-- solid, with nothing on it or reserved.
local HOLE = { [0] = { require = 0x00, forbid = 0x0F8800B0 }, [1] = { require = 0x20, forbid = 0x0F880090 } }
-- The panels a snake aims at (`byte_80D2E28`, any of them): the other
-- side's bodies and other bodies, or a neutral object.
local TARGET = { [0] = 0x05800000, [1] = 0x0A800000 }
-- The ticks to the next snake: in the same column, else.
local SAME_COLUMN, NEXT_COLUMN = 8, 0x18

-- `sub_80D2D40`: the panel a snake from (hx, hy) aims at: of the panels
-- from the column in front of the hole forward (rows 1 to 3, until the
-- column leaves the field) that hold a target, the first in that order,
-- replaced by one in a nearer row, then by one in a nearer column, then
-- (a slip: the candidate's column against the best's row) by one whose
-- column isn't past the best's row. 0xFF when none.
local function target(me: Object, hx: number, hy: number): (number, number)
    local mask = TARGET[me.alliance]
    local candidates: { { number } } = {}
    local x = hx + me.facing
    while true do
        local stop = false
        for y = 1, 3 do
            if not field.valid(x, y) then
                stop = true
                break
            end
            if bit32.btest(field.flags(x, y), mask) then
                table.insert(candidates, { x, y })
            end
        end
        if stop then
            break
        end
        x += me.facing
    end
    if #candidates == 0 then
        return 0xFF, 0
    end
    local bx, by = 0, 0
    for _, c in candidates do
        local cx, cy = c[1], c[2]
        local take = true
        if bx ~= 0 then
            local d, bd = math.abs(cy - hy), math.abs(by - hy)
            if d > bd then
                take = false
            elseif d == bd then
                local e, be = math.abs(cx - hx), math.abs(bx - hx)
                if e > be then
                    take = false
                elseif e == be then
                    take = cx <= by
                end
            end
        end
        if take then
            bx, by = cx, cy
        end
    end
    return bx, by
end

-- `sub_80D2CBE`: from the scan's place on (rows 1 to 3, then the next
-- column back), the first hole; the scan resumes past it. The scan gives
-- up once its column, plus one, passes 7 (a side 0 nest scans columns 6 to
-- 1, then 0 and 0xFF off the field; side 1's, 1 to 6). nil when none, else
-- the hole and its target.
local function next_hole(me: Object): (number?, number, number, number)
    local s = me.state :: State
    local cond = HOLE[me.alliance]
    local step = -me.facing
    local found = false
    local hx, hy = 0, 0
    while true do
        if found then
            break
        end
        if field.check(s.scan_x, s.scan_y, cond.require, cond.forbid) then
            found, hx, hy = true, s.scan_x, s.scan_y
        end
        s.scan_y += 1
        if s.scan_y > 3 then
            s.scan_y = 1
            local x = s.scan_x + step
            s.scan_x = x
            if x + 1 > 7 or x + 1 < 0 then
                break
            end
        end
    end
    if not found then
        return nil, 0, 0, 0
    end
    local tx, ty = target(me, hx, hy)
    return hx, hy, tx, ty
end

-- `sub_80D2C40`: the scan's start (the far column, the top row); a snake
-- out of each hole, and a peek at the next for the wait; once the holes
-- run out, wait for the snakes, then go.
local function tick(me: Object)
    local s = me.state :: State
    if me.phase_init == START then
        me.timer = 1
        s.scan_x = 6 - 5 * bit32.bxor(me.alliance, me.flip)
        s.scan_y = 1
        me.phase_init = SCANNING
    end
    if me.phase_init == SCANNING then
        local t = me.timer - 1
        me.timer = t
        if t ~= 0 then
            return
        end
        local hx, hy, tx, ty
        if field.valid(s.scan_x, s.scan_y) then
            hx, hy, tx, ty = next_hole(me)
        end
        if hx then
            local o = snake.spawn(me, hx, hy, tx, ty)
            if o then
                o.run_while_dimmed = true
            end
            -- The next hole's column decides the wait; the scan's place is
            -- kept.
            local x, y = s.scan_x, s.scan_y
            local next_x = next_hole(me)
            s.scan_x, s.scan_y = x, y
            me.timer = if next_x == hx then SAME_COLUMN else NEXT_COLUMN
            return
        end
        me.phase_init = WAITING
    end
    if s.snakes == 0 then
        me:set_lifecycle("destroy")
    end
end

nest.kind = define.kind {
    id = "snake/nest",
    pool = "attack",
    state = { scan_x = "u8", scan_y = "u8", snakes = "u8", holder = "object" },
    -- `sub_80D2BDC`.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            -- sub_80D2C00: on its panel; on at once.
            me:set_coordinates_from_panel()
            me:set_lifecycle("update")
            tick(me)
        elseif l == "update" then
            tick(me)
        else
            -- sub_80D2E86: the holder goes on; freed.
            local holder = (me.state :: State).holder
            if holder and holder.active then
                (holder.state :: { busy: boolean }).busy = false
            end
            me:free()
            return
        end
        me:update_sprite()
    end,
}

-- `sub_80D2E94`: a nest on (`x`, `y`) of `controller`'s side and facing,
-- with the damage word `damage`, running while dimmed; `holder` (an
-- object with a boolean `busy`) goes on once it is done. (Spawned at the
-- spawner's registers: its init places its X and Y, and its Z stays `z`,
-- what Snake's controller leaves there: its bonus, as a raw 16.16 value.)
function nest.spawn(controller: Object, x: number, y: number, z: number, damage: number, holder: Object): Object?
    local o = battle.spawn(nest.kind, Vec3.new(0, 0, z))
    if o then
        o.panel_x, o.panel_y = x, y
        o:set_damage_word(damage)
        o.related1 = controller
        local s = o.state :: State
        s.holder = holder
        o.alliance, o.flip = controller.alliance, controller.flip
        o.run_while_dimmed = true
    end
    return o
end

return nest
