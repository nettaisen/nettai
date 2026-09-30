--!strict
-- A riding hit (attack object #0x69, `sub_80D18D8`): an unseen hit region
-- on its owner's position, following it panel to panel while the owner
-- stays in the action it was spawned in; SlashMan's rolling slash
-- (RSlash). Param1 is its hit modifier, Param2 its hit spark; with Param3
-- set it keeps going after hitting a body (its region cleared until it
-- reaches another panel), else it ends. [unverified: no lab scenario
-- reaches it; docs/engine/standard-chips.md]

export type State = {
    -- Its collision types (ExtraVars+0, +4).
    self_type: number,
    target: number,
    -- The owner's action it lasts through (ExtraVars+8).
    owner_action: number,
}

local riding_hit = {
    state = { self_type = "u8", target = "u8", owner_action = "u8" },
}

-- What it counts as hitting a body, by side (`byte_80D19B8`).
local BODIES = { [0] = 0x0580_0000, 0x0A80_0000 }

-- `sub_80D19D4`: a riding hit for `owner`, with collision types
-- `self_type` and `target`, the damage word `damage` and `params`. It runs
-- while dimmed; its position (the registers) is the owner's once its init
-- runs.
function riding_hit.spawn(owner: Object, self_type: number, target: number, element: number, params: { number }, damage: number): Object?
    local o = battle.spawn_kind("riding-hit", Vec3.new(target, element, 0), params)
    if o then
        local s = o.state :: State
        s.self_type = self_type
        s.target = target
        s.owner_action = owner.action
        o.element = element
        o.damage = damage
        o.stamina = bit32.rshift(damage, 16)
        o.alliance, o.flip = owner.alliance, owner.flip
        o.related1 = owner
        o.run_while_dimmed = true
    end
    return o
end

-- `sub_80D1A00`: end it (its next update frees it).
function riding_hit.stop(o: Object)
    o:set_lifecycle("destroy")
end

-- Ending: its region cleared, then the destroy state.
local function finish(me: Object)
    me.collision.region = 0
    me:set_lifecycle("destroy")
end

-- `sub_80D1934`: resolve its hits; follow the owner.
local function tick(me: Object)
    local c = me.collision
    c:remove()
    me:hit_spark()
    local s = me.state :: State
    local owner = me.related1 :: Object
    if battle.over() or owner.action ~= s.owner_action then
        return finish(me)
    end
    if bit32.btest(c.hit_flags, BODIES[me.alliance]) then
        if me:param(3) == 0 then
            return finish(me)
        end
        c.region = 0
    end
    local x, y = me.panel_x, me.panel_y
    if not field.valid(x, y) then
        return finish(me)
    end
    me.pos = owner.pos
    me:set_panel_from_coordinates()
    me:update_collision_panels()
    if me.panel_x ~= x or me.panel_y ~= y then
        c.region = 1
    end
    c:present()
end

-- `sub_80D18F8`: on the owner, collision on.
local function init(me: Object)
    me.pos = (me.related1 :: Object).pos
    me:set_panel_from_coordinates()
    if not me:create_collision() then
        me:free()
        return
    end
    local s = me.state :: State
    me:setup_collision(s.self_type, s.target, me:param(1))
    me.collision.hit_effect = me:param(2)
    me.collision:present()
    me:set_lifecycle("update")
end

-- `sub_80D19C0`: resolve its last hits, then go.
local function destroy(me: Object)
    local c = me.collision
    c:remove()
    me:hit_spark()
    c.region = 0
    me:destroy()
end

-- `sub_80D18D8`.
function riding_hit.update(me: Object)
    local l = me.lifecycle
    if l == "init" then
        init(me)
    elseif l == "update" then
        tick(me)
    else
        destroy(me)
    end
end

return riding_hit
