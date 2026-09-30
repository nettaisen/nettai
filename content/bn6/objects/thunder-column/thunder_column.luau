--!strict
-- A thunder column (attack object #0x8B, `sub_80D6924`): an unseen hit on
-- one panel, 16 pixels up, for the ticks its spawner gives it (its timer),
-- or until it has hit something. DolThdr's doll strikes a row of them.
-- Its first parameter picks its collision (`byte_80D6914`): 0 is a plain
-- hit, 1 a pull (collision type 0xB). It registers in its spawner's
-- `columns` list (the game's slot pointer in its first related object)
-- and leaves it when it goes.

local thunder_column = {}

export type State = {
    -- Its spawner, and its place in the spawner's `columns` list.
    owner: Object?,
    link: number,
}

thunder_column.state = { owner = "object", link = "u8" }

-- What a spawner that registers columns keeps.
export type Owner = { columns: { Object? } }

-- By its first parameter (`byte_80D6914`): self and target collision
-- types, hit effect, region, hit modifier, status, bug and its argument.
local KINDS = {
    [0] = { self_type = 4, target = 5, hit_effect = 3, region = 1, hit_mod = 3, status = 0, bug = 0, bug_arg = 0 },
    [1] = { self_type = 0x0B, target = 5, hit_effect = 0, region = 1, hit_mod = 3, status = 0, bug = 0, bug_arg = 0 },
}

-- `sub_80D69E2`: a column on (x, y) for `from`'s side with the damage word
-- `damage` and parameters `params`, registered as `owner`'s columns[link].
-- It stands 16 pixels up (its spawner's Z register).
function thunder_column.spawn(
    from: Object,
    x: number,
    y: number,
    element: number,
    params: { number },
    damage: number,
    owner: Object,
    link: number
): Object?
    local o = battle.spawn_kind("thunder-column", Vec3.new(y, element, 0x10_0000), params)
    if o then
        o.panel_x, o.panel_y = x, y
        o.element = element
        o.damage = damage
        o.stamina = bit32.rshift(damage, 16)
        o.alliance, o.flip = from.alliance, from.flip
        local s = o.state :: State
        s.owner = owner
        s.link = link
    end
    return o
end

-- Its place in its owner's list: `me` there, or empty.
local function register(me: Object, o: Object?)
    local s = me.state :: State
    local owner = s.owner :: Object
    local list = (owner.state :: Owner).columns
    list[s.link] = o
end

-- `sub_80D69B8`: resolve its hits; gone once it hit something or its time
-- is up.
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

-- `sub_80D6952`.
local function init(me: Object)
    local kind = KINDS[me:param(1)]
    if kind == nil then
        error(string.format("thunder column kind %d reads past byte_80D6914", me:param(1)))
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
    me:setup_collision(kind.self_type, kind.target, kind.hit_mod)
    local c = me.collision
    c.region = kind.region
    c.hit_effect = kind.hit_effect
    if kind.status ~= 0 then
        c.status_base = kind.status
    end
    if kind.bug ~= 0 then
        c.bugs = kind.bug + kind.bug_arg * 0x100
    end
    register(me, me)
    c:present()
    me:set_lifecycle("update")
    tick(me)
end

-- `sub_80D6924`.
function thunder_column.update(me: Object)
    local l = me.lifecycle
    if l == "init" then
        init(me)
    elseif l == "update" then
        tick(me)
    else
        -- `sub_80D6944`.
        register(me, nil)
        me:destroy()
    end
end

return thunder_column
