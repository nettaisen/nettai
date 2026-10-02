--!strict
-- FlmHook's hook (the original's effect object #0x8C, `sub_80EA010`): it
-- has no sprite; it swings two flames (./fire), one from the column in
-- front of its user and 20 ticks later one from the column two ahead, then
-- goes 10 ticks later. FlmHook1 to 3 bring it as their instant effect
-- (`sub_80EA11C`, instant chip effect 14); the BigHook Giga chip brings it
-- as its navi (`off_802CD5C[23]`, the same routine), its flames the big
-- ones, from columns 4 then 5 (side 1: 3 then 2), and the controller waits
-- for it. See docs/engine/chips.md §3.6.35.

local fire = require("./fire")

local hook = {}

-- What a chip's hook swings: big flames (from fixed columns), and their
-- hit spark.
export type Flames = { big: boolean, spark: Spark }

export type State = {
    big: boolean,
    spark: Spark?,
    -- The controller waiting for BigHook's (ExtraVars[1]).
    controller: Object?,
}

-- The rows the flames start from; the phases' ticks.
local FIRST_ROW, SECOND_ROW = 4, 0
local SWING_TICKS, END_TICKS = 0x14, 0xA

-- The phases.
local FIRST, SECOND, DONE = 0, 4, 8

-- The controller calls the spawner (`sub_80EA11C`) through r3, which the
-- hook's position takes (nothing sets it later).
local SPAWNER_ADDRESS = 0x080EA11D

-- A flame of swing `swing` on column `x`.
local function flame(me: Object, x: number, row: number, swing: number)
    local s = me.state :: State
    local damage = bit32.bor(me.damage, bit32.lshift(me.stamina, 16))
    fire.swing(me, x, row, me.element, damage, { swing = swing, big = s.big, spark = s.spark :: Spark })
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

-- `sub_80EA05C`: the first flame.
local function first(me: Object)
    if me.phase_init == 0 then
        local x = if (me.state :: State).big then 4 - me.alliance else me.panel_x + me.facing
        flame(me, x, FIRST_ROW, 0)
        me.timer = SWING_TICKS
        me.phase_init = 4
    end
    wait(me, SECOND)
end

-- `sub_80EA0A0`: the second.
local function second(me: Object)
    if me.phase_init == 0 then
        local x = if (me.state :: State).big then 5 - me.alliance * 3 else me.panel_x + 2 * me.facing
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
    id = "bn6:flmhook/hook",
    pool = "effect",
    state = { big = "bool", spark = "spark", controller = "object" },
    -- `sub_80EA010`.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            me:set_lifecycle("update")
            tick(me)
        elseif l == "update" then
            tick(me)
        else
            -- sub_80EA10A: through ExtraVars[1], 0: the controller (if it
            -- waits) moves on (FlmHook's write lands outside RAM).
            local controller = (me.state :: State).controller
            if controller then
                navi_chip.navi_left(controller)
            end
            me:free()
        end
    end,
}

-- `sub_80EA11C`: a hook of `user`'s on its panel (its position the
-- registers', `pos`); a big one's controller waits for it. (The game also
-- keeps the panel in the header's two spare bytes, which nothing reads.)
local function spawn(user: Object, x: number, y: number, element: number, damage: number, pos: Vec3, flames: Flames, controller: Object?)
    local o = battle.spawn(hook.kind, pos)
    if o then
        o.panel_x, o.panel_y = x, y
        o.element = element
        o.alliance, o.flip = user.alliance, user.flip
        o:set_damage_word(damage)
        o.related1 = user
        local s = o.state :: State
        s.big, s.spark = flames.big, flames.spark
        if flames.big then
            s.controller = controller
        end
    end
    return o
end

-- FlmHook's instant effect (`off_80EC3F0[14]`): the hook with `flames`.
-- (A big one's flag write goes through a ROM pointer here: nothing.)
function hook.instant(flames: Flames): (user: Object, spec: InstantChipSpec) -> ()
    return function(user: Object, spec: InstantChipSpec)
        local pos = Vec3.new(spec.panel_y, spec.element, spec.z)
        spawn(user, spec.panel_x, spec.panel_y, spec.element, spec.damage, pos, flames, nil)
    end
end

-- BigHook's navi hook (`off_802CD5C[23]`): the hook with `flames`,
-- reporting to `controller`.
function hook.summon(flames: Flames): (user: Object, controller: Object, spec: NaviChipSpec) -> Object?
    return function(user: Object, controller: Object, spec: NaviChipSpec): Object?
        local pos = Vec3.new(spec.panel_y, spec.element, SPAWNER_ADDRESS)
        return spawn(user, spec.panel_x, spec.panel_y, spec.element, spec.damage, pos, flames, controller)
    end
end

return hook
