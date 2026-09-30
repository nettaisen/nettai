--!strict
-- One-tick hit regions (attack object #3, `object_spawnCollisionRegion`).
-- Most attacks use one: it registers on its panels, resolves against
-- whatever is there, and (by default) frees itself within its first
-- update. See field-collision-damage.md §3.5.

export type State = {
    hit_mod: number,
    status: number,
    bug: number,
    bug_arg: number,
}

-- What a hitbox does.
export type Spec = {
    panel_x: number,
    panel_y: number,
    element: number,
    z: number?,
    -- Region shape.
    region: number,
    hit_effect: number,
    -- Collision type indices.
    target: number,
    self_type: number,
    -- The damage word (damage | flag bits) and the counter byte.
    damage: number,
    stamina: number?,
    hit_mod: number?,
    status: number?,
    bug: number?,
    bug_arg: number?,
}

local hitbox = {
    name = "hitbox",
    pool = "attack",
    index = 3,
    state = { hit_mod = "u8", status = "u8", bug = "u8", bug_arg = "u8" },
}

-- `object_spawnCollisionRegion`, spawned by `owner`.
function hitbox.spawn(owner: Object, spec: Spec): Object?
    -- The spawn position is the caller's registers (panel y, element, z)
    -- until init places it.
    local pos = Vec3.new(spec.panel_y, spec.element, spec.z or 0)
    local o = battle.spawn("attack", hitbox.index, pos, { spec.region, spec.hit_effect, spec.target, spec.self_type })
    if not o then
        return nil
    end
    o.panel_x, o.panel_y = spec.panel_x, spec.panel_y
    o.element = spec.element
    o.damage = spec.damage
    o.stamina = spec.stamina or 0
    o.alliance, o.flip = owner.alliance, owner.flip
    local s = o.state :: State
    s.hit_mod = spec.hit_mod or 0
    s.status = spec.status or 0
    s.bug = spec.bug or 0
    s.bug_arg = spec.bug_arg or 0
    return o
end

-- `sub_80C532E`.
local function resolve(me: Object)
    local c = me.collision
    c:remove()
    me:hit_spark()
    if c.hit_flags == 0 then
        local left = me.timer - 1
        me.timer = left
        if left > 0 then
            c:present()
            return
        end
    end
    c.region = 0
    c:free()
    me:free()
end

-- `sub_80C52D0`.
local function init(me: Object)
    local x, y = me.panel_x, me.panel_y
    if not battle.panel_valid(x, y) then
        me:free()
        return
    end
    local cx, cy = battle.panel_center(x, y)
    me.pos = Vec3.new(cx, cy, me.pos.z)
    if not me:create_collision() then
        me:free()
        return
    end
    local s = me.state :: State
    me:setup_collision(me:param(4), me:param(3), s.hit_mod)
    local c = me.collision
    c.region = me:param(1)
    c.hit_effect = me:param(2)
    if s.status ~= 0 then
        c.status_base = s.status
    end
    if s.bug ~= 0 then
        c.bugs = s.bug_arg * 0x100 + s.bug
    end
    c:present()
    me:set_lifecycle("update")
    resolve(me)
end

function hitbox.update(me: Object)
    local l = me.lifecycle
    if l == "init" then
        init(me)
    elseif l == "update" then
        resolve(me)
    else
        me:destroy()
    end
end

return hitbox
