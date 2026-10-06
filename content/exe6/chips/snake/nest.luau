--!strict
-- Snake's nest (the original's attack object #0x72, `sub_80D2BDC`;
-- docs/engine/dimming-chip-effects.md §13): it scans the user's side's
-- holes from the far column back, sending a snake (./snake) out of each
-- at the nearest enemy, 8 ticks apart in a column and 24 between columns;
-- once the snakes are gone, it lets its holder go on. It has no sprite.
-- (The holes' scan and the snakes' targets are the shared snake's,
-- @exelib/snake/snake.)

local common = require("@exelib/snake")
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
-- The ticks to the next snake: in the same column, else.
local SAME_COLUMN, NEXT_COLUMN = 8, 0x18

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
        local hx: number?, hy: number, tx: number, ty: number
        if field.valid(s.scan_x, s.scan_y) then
            hx, hy, tx, ty = common.next_hole(me, s)
        end
        if hx then
            local o = snake.spawn(me, hx, hy, tx, ty)
            if o then
                o.run_while_dimmed = true
            end
            -- The next hole's column decides the wait; the scan's place is
            -- kept.
            local x, y = s.scan_x, s.scan_y
            local next_x = common.next_hole(me, s)
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

nest.kind = new.kind {
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
