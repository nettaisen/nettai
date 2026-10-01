--!strict
-- FlmHook's hook (the original's effect object #0x8C, `sub_80EA010`) and
-- the chips' effect (instant chip effect 14, `sub_80EA11C`): it swings two
-- flames (fire.luau), one from the column in front of the user and 20
-- ticks later one from the column two ahead, then goes 10 ticks later. A
-- fast hook (the original's Param1 set, which no BN6 chip has) swings fast
-- flames from fixed columns instead: 4, then 5 (3, then 2, on side 1). It
-- has no sprite.
--
-- FlmHook1-3 keep the pack's records (they are Program Advances, which
-- FireHit's records make), and run `hook.instant` through
-- chips/146-flmhook1.

local fire = require("./fire")

local hook = {}

-- How a hook swings: fast flames from the fixed columns (the original's
-- Param1), and the flames' hit spark (Param2).
export type Swing = { fast: boolean?, spark: Spark? }

export type State = {
    fast: boolean,
    spark: Spark?,
}

-- The rows the flames start from.
local FIRST_ROW, SECOND_ROW = 4, 0
local SWING_TICKS, END_TICKS = 0x14, 0xA

-- The phases.
local FIRST, SECOND, DONE = 0, 4, 8

-- A flame of swing `swing` on column `x`.
local function flame(me: Object, x: number, row: number, swing: number)
    local s = me.state :: State
    local damage = bit32.bor(me.damage, bit32.lshift(me.stamina, 16))
    fire.spawn(me, x, row, me.element, swing, s.fast, s.spark, damage)
end

-- Wait out the phase, then the next.
local function wait(me: Object, next: number)
    local left = me.timer - 1
    me.timer = left
    if left <= 0 then
        me.phase = next
        me.phase_init = 0
    end
end

-- `sub_80EA05C`.
local function first(me: Object)
    if me.phase_init == 0 then
        local x = if (me.state :: State).fast then 4 - me.alliance else me.panel_x + me.facing
        flame(me, x, FIRST_ROW, 0)
        me.timer = SWING_TICKS
        me.phase_init = 4
    end
    wait(me, SECOND)
end

-- `sub_80EA0A0`.
local function second(me: Object)
    if me.phase_init == 0 then
        local x = if (me.state :: State).fast then 5 - me.alliance * 3 else me.panel_x + 2 * me.facing
        flame(me, x, SECOND_ROW, 1)
        me.timer = SWING_TICKS
        me.phase_init = 4
    end
    wait(me, DONE)
end

-- `sub_80EA0EC`.
local function done(me: Object)
    if me.phase_init == 0 then
        me.timer = END_TICKS
        me.phase_init = 4
    end
    local left = me.timer - 1
    me.timer = left
    if left <= 0 then
        me:set_lifecycle("destroy")
    end
end

-- `sub_80EA03C`: by phase.
local function tick(me: Object)
    local p = me.phase
    if p == FIRST then
        first(me)
    elseif p == SECOND then
        second(me)
    elseif p == DONE then
        done(me)
    else
        error(string.format("FlmHook's hook phase %#x reads past its table", p))
    end
end

hook.kind = define.kind {
    id = "flmhook/hook",
    pool = "effect",
    state = { fast = "bool", spark = "spark" },
    -- `sub_80EA010`.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            me:set_lifecycle("update")
            tick(me)
        elseif l == "update" then
            tick(me)
        else
            -- sub_80EA10A (its write through ExtraVars+4 lands outside RAM).
            me:free()
        end
    end,
}

-- `sub_80EA11C`, instant chip effect 14: an `instant` hook that sets a hook
-- of the user's on its panel, swinging as `swing` says. (It spawns at the
-- registers' values. The game also keeps the panel in the header's two
-- spare bytes, which nothing reads, and for a fast hook writes to the ROM
-- through a stale register, which does nothing.)
function hook.instant(swing: Swing): (user: Object, spec: InstantChipSpec) -> ()
    return function(user: Object, spec: InstantChipSpec)
        local o = battle.spawn(hook.kind, Vec3.new(spec.panel_y, spec.element, spec.z))
        if o then
            o.panel_x, o.panel_y = spec.panel_x, spec.panel_y
            o.element = spec.element
            o.alliance, o.flip = user.alliance, user.flip
            o:set_damage_word(spec.damage)
            o.related1 = user
            local s = o.state :: State
            s.fast, s.spark = swing.fast == true, swing.spark
        end
    end
end

return hook
