--!strict
-- A hit zone (attack object #0x8B, `sub_80D6924`): an invisible hit region
-- that stays on a panel until it hits something or its timer (which its
-- spawner sets) runs out. Its kind (the first parameter) is data,
-- `data.objects.hit_zones`: collision types, region, hit effect, status
-- and bug. EraseCross's beam (objects/erase-ray) lays them; meteors and
-- other chips' objects do too.
--
-- The game gives it a pointer to a word in its spawner, where it puts
-- itself while it lives: here the spawner and an index into the spawner's
-- state array `zones` (1-based there).

export type State = {
    -- The spawner that tracks it, and where (`zones[slot + 1]`).
    owner: Object?,
    slot: number,
}

local hit_zone = {
    state = { owner = "object", slot = "u8" },
}

-- Its place in the spawner's `zones` (nil when it has none).
local function track(s: State, me: Object?)
    local owner = s.owner
    if owner then
        (owner.state :: any).zones[s.slot + 1] = me
    end
end

-- `sub_80D69E2`: a zone of `spawner`'s on (x, y) with `params` (its kind
-- first), the damage word `damage` and height `z`, tracked in
-- `owner.state.zones[slot + 1]` if `owner` is given. (Its X and Y are its
-- spawner's registers until its init places it.)
function hit_zone.spawn(
    spawner: Object,
    x: number,
    y: number,
    element: number,
    z: number,
    params: { number },
    damage: number,
    owner: Object?,
    slot: number
): Object?
    local o = battle.spawn_kind("hit-zone", Vec3.new(0, 0, z), params)
    if o then
        o.panel_x, o.panel_y = x, y
        o.element = element
        o.damage = bit32.band(damage, 0xFFFF)
        o.stamina = bit32.rshift(damage, 16)
        o.alliance, o.flip = spawner.alliance, spawner.flip
        local s = o.state :: State
        s.owner = owner
        s.slot = slot
    end
    return o
end

-- `sub_80D6A02`: the same, running while dimmed.
function hit_zone.spawn_while_dimmed(
    spawner: Object,
    x: number,
    y: number,
    element: number,
    z: number,
    params: { number },
    damage: number,
    owner: Object?,
    slot: number
): Object?
    local o = hit_zone.spawn(spawner, x, y, element, z, params, damage, owner, slot)
    if o then
        o.run_while_dimmed = true
    end
    return o
end

-- `sub_80D69B8`: resolve its hits; end on a hit or when the timer runs
-- out, else register again.
local function tick(me: Object)
    local c = me.collision
    c:remove()
    me:hit_spark()
    if c.hit_flags == 0 then
        local t = me.timer - 1
        me.timer = t
        if t > 0 then
            c:present()
            return
        end
    end
    c.region = 0
    me:set_lifecycle("destroy")
end

-- `sub_80D6952`: on its panel with its kind's registration; then it
-- tracks itself in its spawner.
local function init(me: Object)
    local kind = me:param(1)
    local z = data.objects.hit_zones[kind]
    if z == nil then
        error(string.format("hit zone kind %#x reads past byte_80D6914 (sub_80D6952)", kind))
    end
    if not field.valid(me.panel_x, me.panel_y) then
        me:free()
        return
    end
    me:set_coordinates_from_panel()
    if not me:create_collision() then
        me:free()
        return
    end
    me:setup_collision(z.self_type, z.target_type, z.hit_mod)
    local c = me.collision
    c.region = z.region
    c.hit_effect = z.hit_effect
    if z.status ~= 0 then
        c.status_base = z.status
    end
    if z.bug ~= 0 then
        c.bugs = z.bug + z.bug_arg * 0x100
    end
    track(me.state :: State, me)
    c:present()
    me:set_lifecycle("update")
    tick(me)
end

-- `sub_80D6924`.
function hit_zone.update(me: Object)
    local l = me.lifecycle
    if l == "init" then
        init(me)
    elseif l == "update" then
        tick(me)
    else
        -- sub_80D6944
        track(me.state :: State, nil)
        me:destroy()
    end
end

return hit_zone
