--!strict
-- A drill (attack object #0x71, `sub_80D2A94`): DrilArm's. It sits on the
-- panel ahead of its owner (following it), invisible, hitting what stands
-- there every 12 ticks for its second parameter's ticks; a hit takes its
-- region away until the next 12. The first one (first parameter 0) brings
-- a second drill on the panel ahead of itself. Each goes as soon as the
-- slot that holds it lets go of it: its navi's first related object, or,
-- for the second, the first drill's link, which the first clears when it
-- goes. A hit also shakes the camera.

export type State = {
    -- Where it is held (ExtraVars+0, a pointer in the game): its owner's
    -- first related object, or its owner's second drill.
    slot: "related" | "child",
    -- The second drill (ExtraVars+8).
    child: Object?,
}

local drill = {
    state = { slot = { "related", "child" }, child = "object" },
}

-- Ticks between hits.
local PERIOD = 0x0C
-- Collision types: the first drill's and the second's; hit modifiers.
local FIRST_TYPE, SECOND_TYPE, TARGET_TYPE = 6, 0x4A, 5
local HIT_MOD, LIGHT_HIT_MOD = 0x61, 3

-- `sub_80D2B8E`: a drill for `owner` with the damage word `damage` and
-- `params`, at height `z` (the rest of its position is the spawner's
-- registers until its init places it), held in `slot` of `owner`.
function drill.spawn(
    owner: Object,
    element: number,
    z: number,
    damage: number,
    params: { number },
    slot: "related" | "child"
): Object?
    local o = battle.spawn_kind("drill", Vec3.new(0, element, z), params)
    if o then
        o.element = element
        o.alliance, o.flip = owner.alliance, owner.flip
        o.related1 = owner
        o.damage = damage
        o.stamina = bit32.rshift(damage, 16)
        local s = o.state :: State
        s.slot = slot
        if slot == "related" then
            owner.related1 = o
        else
            (owner.state :: State).child = o
        end
    end
    return o
end

-- Whether the slot that holds it still does.
local function held(me: Object): boolean
    local s = me.state :: State
    local owner = me.related1 :: Object
    if s.slot == "related" then
        return owner.related1 == me
    end
    return (owner.state :: State).child == me
end

-- `sub_80D2BC0`: onto the panel ahead of its owner.
local function follow(me: Object)
    local owner = me.related1 :: Object
    me.panel_x = owner.panel_x + me.facing
    me.panel_y = owner.panel_y
    me:set_coordinates_from_panel()
    me:update_collision_panels()
end

-- `sub_80D2B2C`: resolve its hits; count down; follow.
local function tick(me: Object)
    local c = me.collision
    c:remove()
    me:hit_spark()
    local done = not held(me)
    if not done then
        if c.hit_flags ~= 0 then
            c.region = 0
            battle.shake_camera(0, 0xC)
        end
        local left = me.timer2 - 1
        me.timer2 = left
        if left < 0 then
            done = true
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
    if done then
        c.region = 0;
        (me.state :: State).child = nil
        me:set_lifecycle("destroy")
    end
    follow(me)
    c:present()
end

-- `sub_80D2AB4`: the first brings the second; then on its panel.
local function init(me: Object)
    local s = me.state :: State
    local first = me:param(1) == 0
    if first then
        local params = { me:param(1) + 1, me:param(2), me:param(3), me:param(4) }
        local damage = me.damage + me.stamina * 0x10000
        local second = drill.spawn(me, me.element, me.pos.z, damage, params, "child")
        if second then
            second.run_while_dimmed = me.run_while_dimmed
        end
    end
    if not me:create_collision() then
        s.child = nil
        me:free()
        return
    end
    local hit_mod = if me:param(3) == 1 then LIGHT_HIT_MOD else HIT_MOD
    me:setup_collision(if first then FIRST_TYPE else SECOND_TYPE, TARGET_TYPE, hit_mod)
    me.timer2 = me:param(2)
    me.timer = PERIOD
    follow(me)
    local c = me.collision
    c.region = 1
    c.hit_effect = 0x0A
    me:set_lifecycle("update")
    tick(me)
end

-- `sub_80D2A94`.
function drill.update(me: Object)
    local l = me.lifecycle
    if l == "init" then
        init(me)
    elseif l == "update" then
        tick(me)
    else
        me:destroy()
    end
end

return drill
