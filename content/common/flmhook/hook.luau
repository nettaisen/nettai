--!strict
-- The hook, as BN6 and BN5 share it (BN6's effect object #0x8C,
-- `sub_80EA010`, FlmHook's and BigHook's; BN5's #0x8C, 0x080EA8A0,
-- BigHook's): it has no sprite; it swings two flames (./fire), the first
-- from a column ahead of its user and 20 ticks later the second from
-- another, then goes 10 ticks later. FlmHook1 to 3 bring it as their
-- instant effect (BN6's `sub_80EA11C`, instant chip effect 14); the BigHook
-- Giga chip brings it as its navi, its flames the big ones from fixed
-- columns, and the controller waits for it. Each game makes its kind
-- (`hook.make`: its id, its flames' columns, its flames, and whether it
-- keeps its user's element). See docs/engine/chips.md §3.6.35.

local hook = {}

-- What a chip's hook swings: big flames (from fixed columns), and their
-- hit spark.
export type Flames = { big: boolean, spark: Spark }

-- A game's hook.
export type Spec = {
    id: string,
    -- The columns of its flames (the first's, the second's).
    columns: (me: Object, big: boolean) -> (number, number),
    -- A flame of swing `swing` from (x, y): the game's.
    flame: (owner: Object, x: number, y: number, element: number, damage: number, swing: number, flames: Flames) -> Object?,
    -- BN6's keeps its user's element (BN5's spawner leaves it 0).
    keeps_element: boolean,
}

export type State = {
    big: boolean,
    spark: Spark?,
    -- The controller waiting for BigHook's (BN6's ExtraVars[1], BN5's
    -- +0x64).
    controller: Object?,
}

-- The rows the flames start from; the phases' ticks.
local FIRST_ROW, SECOND_ROW = 4, 0
local SWING_TICKS, END_TICKS = 0x14, 0xA

-- The phases.
local FIRST, SECOND, DONE = 0, 4, 8

-- A game's hook: its kind, FlmHook's instant effect and BigHook's navi hook
-- (the controller calls the spawner through r3, `spawner_address`, which
-- the hook's position takes).
function hook.make(spec: Spec): {
    kind: Kind,
    instant: (flames: Flames) -> (user: Object, spec: InstantChipSpec) -> (),
    summon: (flames: Flames, spawner_address: number) -> (user: Object, controller: Object, spec: NaviChipSpec) -> Object?,
}
    -- A flame of swing `swing` on column `x`.
    local function flame(me: Object, x: number, row: number, swing: number)
        local s = me.state :: State
        local damage = bit32.bor(me.damage, bit32.lshift(me.stamina, 16))
        spec.flame(me, x, row, me.element, damage, swing, { big = s.big, spark = s.spark :: Spark })
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
            local x = spec.columns(me, (me.state :: State).big)
            flame(me, x, FIRST_ROW, 0)
            me.timer = SWING_TICKS
            me.phase_init = 4
        end
        wait(me, SECOND)
    end

    -- `sub_80EA0A0`: the second.
    local function second(me: Object)
        if me.phase_init == 0 then
            local _, x = spec.columns(me, (me.state :: State).big)
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
            error(string.format("The hook's phase %#x reads past its table", p))
        end
    end

    local kind = define.kind {
        id = spec.id,
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
                -- sub_80EA10A: through its controller's flag, 0: the
                -- controller (if it waits) moves on (FlmHook's write lands
                -- outside RAM).
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
    local function spawn(
        user: Object,
        x: number,
        y: number,
        element: number,
        damage: number,
        pos: Vec3,
        flames: Flames,
        controller: Object?
    ): Object?
        local o = battle.spawn(kind, pos)
        if o then
            o.panel_x, o.panel_y = x, y
            if spec.keeps_element then
                o.element = element
            end
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
    local function instant(flames: Flames): (user: Object, spec: InstantChipSpec) -> ()
        return function(user: Object, ispec: InstantChipSpec)
            local pos = Vec3.new(ispec.panel_y, ispec.element, ispec.z)
            spawn(user, ispec.panel_x, ispec.panel_y, ispec.element, ispec.damage, pos, flames, nil)
        end
    end

    -- BigHook's navi hook: the hook with `flames`, reporting to
    -- `controller`.
    local function summon(flames: Flames, spawner_address: number): (user: Object, controller: Object, spec: NaviChipSpec) -> Object?
        return function(user: Object, controller: Object, nspec: NaviChipSpec): Object?
            local pos = Vec3.new(nspec.panel_y, nspec.element, spawner_address)
            return spawn(user, nspec.panel_x, nspec.panel_y, nspec.element, nspec.damage, pos, flames, controller)
        end
    end

    return { kind = kind, instant = instant, summon = summon }
end

return hook
