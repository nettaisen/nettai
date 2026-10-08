--!strict
-- A thunder column (attack object #0x8B, `sub_80D6924`): an unseen hit on
-- one panel, 16 pixels up, for the ticks its spawner gives it (its timer),
-- or until it has hit something. DolThdr's doll (./doll) strikes a row of
-- them; EraseCross's ray (navis/megaman/forms/erasecross/ray, which
-- MstrCros's EraseCross fires too) uses them as well, laying the pulling
-- ones. What it hits with is its variant (a row of the original's
-- `byte_80D6914`, its first parameter): `plain` or `pull`. It registers
-- in its spawner's `columns` list (the game's slot pointer in its first
-- related object) and leaves it when it goes.

local collision = require("../../rules/collision")

local thunder_column = {}

-- What a column hits with: what it is and hits, its spark, region, hit
-- modifier, status, bug and the bug's argument (none: 0).
export type Variant = {
    collision: { is: CollisionType, hits: CollisionType },
    spark: Spark,
    region: Region,
    hit_mod: number,
    status: Status?,
    bug: number,
    bug_arg: number,
}

export type State = {
    -- Its spawner, and its place in the spawner's `columns` list.
    owner: Object?,
    link: number,
    variant: Variant,
}

-- What a spawner that registers columns keeps.
export type Owner = { columns: { Object? } }

-- The height it stands at (its spawner's Z register).
local HEIGHT = 0x10_0000

-- A column's variant (a row of `byte_80D6914`, its user's: DolThdr's plain
-- hit, EraseCross's pull).
function thunder_column.variant(v: Variant): Variant
    return new.record("thunder-column-variant", v)
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
    c:set_region(nil)
    me:set_lifecycle("destroy")
end

-- `sub_80D6952`.
local function init(me: Object)
    local v = (me.state :: State).variant
    if not field.valid(me.panel_x, me.panel_y) then
        me:free()
        return
    end
    me:set_coordinates_from_panel()
    if not me:create_collision() then
        me:free()
        return
    end
    me:setup_collision(v.collision.is, v.collision.hits, v.hit_mod)
    local c = me.collision
    c:set_region(v.region)
    c:set_hit_effect(v.spark)
    if v.status then
        c.status_base = v.status
    end
    if v.bug ~= 0 then
        c.bugs = v.bug + v.bug_arg * 0x100
    end
    register(me, me)
    c:present()
    me:set_lifecycle("update")
    tick(me)
end

thunder_column.kind = new.kind {
    id = "dolthdr/thunder-column",
    pool = "attack",
    state = { owner = "object", link = "u8", variant = "record:thunder-column-variant" },
    -- `sub_80D6924`.
    update = function(me: Object)
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
    end,
}

-- `sub_80D69E2`: a column of `variant` on (x, y) for `from`'s side with
-- the damage word `damage`, registered as `owner`'s columns[link]. It
-- stands 16 pixels up (its spawner's Z register).
function thunder_column.spawn(
    from: Object,
    x: number,
    y: number,
    element: number,
    variant: Variant,
    damage: number,
    owner: Object,
    link: number
): Object?
    local o = battle.spawn(thunder_column.kind, Vec3.new(y, element, HEIGHT))
    if o then
        o.panel_x, o.panel_y = x, y
        o.element = element
        o:set_damage_word(damage)
        o.alliance, o.flip = from.alliance, from.flip
        local s = o.state :: State
        s.owner = owner
        s.link = link
        s.variant = variant
    end
    return o
end

return thunder_column
