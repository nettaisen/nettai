--!strict
-- A drill's hit (attack object #0x71, `sub_80D2A94`): an invisible hit
-- region that stays on the panel in front of whatever spawned it, for the
-- second parameter's ticks, hitting again every 12 ticks (unless the third
-- parameter is 1). One of the first kind (first parameter 0) brings one of
-- the second, which follows it a panel further on. GroundCross's charged
-- shot and DrilArm drill with it.
--
-- The game gives it a pointer to where its spawner keeps it, and ends it
-- once something else is kept there: the spawner's first related slot, or
-- (for the second kind) its first's `drill`.

export type Slot = "related1" | "drill"

export type State = {
    -- Who keeps it and where (none: nothing ends it early).
    holder: Object?,
    slot: Slot,
    -- ExtraVars+8: the second kind it brought.
    drill: Object?,
}

local drill_hit = {
    state = { holder = "object", slot = { "related1", "drill" }, drill = "object" },
}

-- What it is (by kind), what it hits, and its hit modifier (a single hit,
-- or the repeating one).
local SELF_FIRST, SELF_SECOND, TARGET = 6, 0x4A, 5
local HIT_MOD_ONCE, HIT_MOD = 3, 0x61

-- Ticks between its hits.
local PERIOD = 12

local function kept(holder: Object, slot: Slot): Object?
    if slot == "related1" then
        return holder.related1
    end
    return (holder.state :: State).drill
end

local function keep(holder: Object, slot: Slot, o: Object?)
    if slot == "related1" then
        holder.related1 = o
    else
        (holder.state :: State).drill = o
    end
end

-- `sub_80D2B8E`: a drill hit of `spawner`'s (which it follows) with
-- `params`, the damage word `damage` and height `z`, kept by `holder` in
-- `slot` if given. (Its X and Y are its spawner's registers until its
-- init places it.)
function drill_hit.spawn(
    spawner: Object,
    element: number,
    params: { number },
    damage: number,
    z: number,
    holder: Object?,
    slot: Slot
): Object?
    local o = battle.spawn_kind("drill-hit", Vec3.new(0, 0, z), params)
    if o then
        o.element = element
        o.alliance, o.flip = spawner.alliance, spawner.flip
        o.related1 = spawner
        o.damage = bit32.band(damage, 0xFFFF)
        o.stamina = bit32.rshift(damage, 16)
        local s = o.state :: State
        s.holder = holder
        s.slot = slot
        if holder then
            keep(holder, slot, o)
        end
    end
    return o
end

-- `sub_80D2BC0`: on the panel in front of what it follows.
local function follow(me: Object)
    local lead = me.related1 :: Object
    me.panel_x, me.panel_y = lead.panel_x + me.facing, lead.panel_y
    me:set_coordinates_from_panel()
    me:update_collision_panels()
end

-- `sub_80D2B2C`: resolve its hits; end once its holder keeps something
-- else or its time is up; hit again every 12 ticks.
local function tick(me: Object)
    local s = me.state :: State
    local c = me.collision
    c:remove()
    me:hit_spark()
    local holder = s.holder
    local ends = holder ~= nil and kept(holder, s.slot) ~= me
    if not ends then
        if c.hit_flags ~= 0 then
            -- (And a camera shake: presentation only.)
            c.region = 0
        end
        local left = me.timer2 - 1
        me.timer2 = left
        if left < 0 then
            ends = true
        else
            local t = me.timer - 1
            me.timer = t
            if t <= 0 then
                me.timer = PERIOD
                if me:param(3) ~= 1 then
                    c.region = 1
                end
            end
        end
    end
    if ends then
        c.region = 0
        s.drill = nil
        me:set_lifecycle("destroy")
    end
    follow(me)
    c:present()
end

-- `sub_80D2AB4`: the first kind brings the second; then its registration.
local function init(me: Object)
    local s = me.state :: State
    if me:param(1) == 0 then
        local p = { me:param(1) + 1, me:param(2), me:param(3), me:param(4) }
        local damage = bit32.bor(me.damage, bit32.lshift(me.stamina, 16))
        local second = drill_hit.spawn(me, me.element, p, damage, me.pos.z, me, "drill")
        if second then
            second.run_while_dimmed = me.run_while_dimmed
        end
    end
    if not me:create_collision() then
        s.drill = nil
        me:free()
        return
    end
    local self_type = if me:param(1) ~= 0 then SELF_SECOND else SELF_FIRST
    me:setup_collision(self_type, TARGET, if me:param(3) == 1 then HIT_MOD_ONCE else HIT_MOD)
    me.timer2 = me:param(2)
    me.timer = PERIOD
    follow(me)
    local c = me.collision
    c.region = 1
    c.hit_effect = 0xA
    me:set_lifecycle("update")
    tick(me)
end

-- `sub_80D2A94`.
function drill_hit.update(me: Object)
    local l = me.lifecycle
    if l == "init" then
        init(me)
    elseif l == "update" then
        tick(me)
    else
        me:destroy()
    end
end

return drill_hit
