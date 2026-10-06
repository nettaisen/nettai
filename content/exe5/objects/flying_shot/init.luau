--!strict
-- EXE5's flying shot (attack object #0x0B, 0x080C5DCC): EXE5's own, EXE6's (`sub_80C60A8`, objects/flying_shot)
-- without the wait, the fall, the palette, the thrown obstacle and the end effect. It flies forward at its variant's
-- speed until it has passed its range in panel centers, and ends on the first thing it hits or when it leaves the
-- field. Its variant (its first parameter: one of the 16-byte rows at 0x080C5D0C, which each user defines where it
-- fires it: `flying_shot.variant { ... }`) gives its collision, element, spark, status, look, speed and range, and
-- whether it lights its panel.

local collision = require("../rules/collision")

local flying_shot = {}

export type State = {
    -- Its variant (its first parameter).
    variant: FlyingShotVariant,
    -- The panel centers it has left to pass (its third parameter).
    range: number,
    -- It lights its panel (its second parameter).
    highlight: boolean,
}

local SHOT = { is = collision.attack, hits = collision.hits_navis }

-- A flying shot variant: what it is (an attack unless said), its hit modifier, element byte, hit spark (none: none),
-- look, whether it lights its panel, the status its hits carry and its speed. (Every row hits navis, passes 8 panel
-- centers and draws its first part with the sprite.)
function flying_shot.variant(r: {
    is: CollisionType?,
    hit_mod: number,
    element: number,
    spark: Spark?,
    sprite: SpriteAsset,
    anim: number?,
    highlight: boolean?,
    status: Status?,
    speed: number,
}): FlyingShotVariant
    return new.record("flying-shot-variant", {
        is = r.is or SHOT.is,
        hits = SHOT.hits,
        hit_mod = r.hit_mod,
        element = r.element,
        spark = r.spark,
        sprite = r.sprite,
        anim = r.anim or 0,
        highlight = r.highlight or false,
        range = 8,
        shadow = false,
        status = r.status,
        speed = r.speed,
    })
end

-- `sub_800E6E8` (EXE5's 0x0800CB08): `c` lies past `a` and up to `b`, either way.
local function crossed(a: number, b: number, c: number): boolean
    if a <= b then
        return c > a and c <= b
    end
    return c > b and c <= a
end

-- Gone: hidden, its region cleared, destroyed next tick.
local function finish(me: Object)
    me.visible = false
    me.collision:set_region(nil)
    me:set_lifecycle("destroy")
end

-- 0x080C5E80: the hits against it resolve and spark; it ends on a hit, else flies: each panel center passed spends
-- its range, and it ends when that runs out or it leaves the field. A highlighting variant lights its panel.
local function tick(me: Object)
    local s = me.state :: State
    local c = me.collision
    c:remove()
    me:hit_spark()
    if c.hit_flags ~= 0 then
        finish(me)
    else
        local cx = field.center(me.panel_x, me.panel_y)
        local p = me.pos
        local x = int.i32(p.x + me.vel.x)
        me.pos = Vec3.new(x, p.y, p.z)
        local spent = false
        if crossed(x, p.x, cx) then
            local left = s.range - 1
            s.range = left
            spent = left <= 0
        end
        if spent then
            finish(me)
        else
            me:set_panel_from_coordinates()
            me:update_collision_panels()
            if not field.valid(me.panel_x, me.panel_y) then
                finish(me)
            end
        end
    end
    c:present()
    -- `object_highlightCurrentCollisionPanels` (EXE5's 0x0800B388): the panels of its region (none once it ends).
    -- Drawn only.
    if s.highlight then
        me:highlight_collision_panels()
    end
end

-- 0x080C5DF0: its look, element, range, speed and collision from its variant, on the panel under it. (The game
-- overwrites its link to its owner with its row's address, so the owner is forgotten.)
local function init(me: Object)
    local s = me.state :: State
    local r = s.variant
    me.related1 = nil
    local look = me.sprite
    look:load(r.sprite)
    look.shadow = if r.shadow then "ground" else "with_sprite"
    me.anim = r.anim
    me.anim_loaded = 0xFF
    me.visible = true
    look:set_flip(bit32.bxor(me.alliance, me.flip))
    me.element = r.element
    s.range = r.range
    me:set_panel_from_coordinates()
    me.vel = Vec3.new(me.facing * r.speed, me.vel.y, me.vel.z)
    s.highlight = r.highlight
    if not me:create_collision() then
        me:free()
        return
    end
    me:setup_collision(r.is, r.hits, r.hit_mod)
    local c = me.collision
    c:set_hit_effect(r.spark)
    c.status_base = r.status
    c:present()
    me:set_lifecycle("update")
    tick(me)
end

flying_shot.kind = new.kind {
    id = "flying-shot",
    pool = "attack",
    state = { variant = "record:flying-shot-variant", range = "u8", highlight = "bool" },
    -- 0x080C5DCC.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
            if not me.active then
                return
            end
        elseif l == "update" then
            tick(me)
        else
            me:destroy()
            return
        end
        me:update_sprite()
    end,
}

-- 0x080C5EE6: a flying shot of `variant` from `owner` at `pos`, with the damage word; it takes the owner's side and
-- flip, and runs later this tick, right after its spawner.
function flying_shot.launch(owner: Object, pos: Vec3, variant: FlyingShotVariant, damage: number): Object?
    local o = battle.spawn(flying_shot.kind, pos)
    if o then
        (o.state :: State).variant = variant
        o:set_damage_word(damage)
        o.alliance, o.flip = owner.alliance, owner.flip
        o.related1 = owner
    end
    return o
end

return flying_shot
