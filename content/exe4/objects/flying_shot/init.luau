--!strict
-- EXE4's flying shot (attack object #0x0B, 0x080CE6E0): EXE5's (0x080C5DCC, its objects/flying_shot) with each
-- variant's own range, no status on its hits, and an effect where a variant says one shows as its range runs out (row
-- 5's, effect row 0x56 of 0x080E1698). It flies forward at its variant's speed until it has passed its range in panel
-- centers, and ends on the first thing it hits or when it leaves the field. Its variant (its first parameter: one of
-- the 16-byte rows at 0x080CE5B0, which each user defines where it fires it: `flying_shot.variant { ... }`) gives its
-- collision, element, spark, look, speed and range, and whether it lights its panel.

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

-- A flying shot variant (a row of 0x080CE5B0): what it is (an attack unless said) and what it hits (navis unless
-- said), its hit modifier, element byte, hit spark (none: none), look, whether it casts a shadow on the ground and
-- lights its panel, the panel centers it passes, its speed (16.16 a tick, forward), and the effect that shows where
-- its range runs out.
function flying_shot.variant(r: {
    is: CollisionType?,
    hits: CollisionType?,
    hit_mod: number,
    element: number,
    spark: Spark?,
    sprite: SpriteAsset,
    anim: number?,
    shadow: boolean?,
    highlight: boolean?,
    range: number,
    speed: number,
    end_effect: Effect?,
}): FlyingShotVariant
    return new.record("flying-shot-variant", {
        is = r.is or collision.attack,
        hits = r.hits or collision.hits_navis,
        hit_mod = r.hit_mod,
        element = r.element,
        spark = r.spark,
        sprite = r.sprite,
        anim = r.anim or 0,
        shadow = r.shadow or false,
        highlight = r.highlight or false,
        range = r.range,
        speed = r.speed,
        end_effect = r.end_effect,
    })
end

-- 0x0800AE10 (EXE6's `sub_800E6E8`): `c` lies past `a` and up to `b`, either way.
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

-- 0x080CE78E: the hits against it resolve and spark; it ends on a hit, else flies: each panel center passed spends
-- its range, and when that runs out it ends, its variant's effect showing at the panel's center at its height; it
-- ends too when it leaves the field. A highlighting variant lights its panel.
local function tick(me: Object)
    local s = me.state :: State
    local c = me.collision
    c:remove()
    me:hit_spark()
    if c.hit_flags ~= 0 then
        finish(me)
    else
        local cx, cy = field.center(me.panel_x, me.panel_y)
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
            local effect = s.variant.end_effect
            if effect then
                battle.effect(Vec3.new(cx, cy, me.pos.z), effect)
            end
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
    -- `object_highlightCurrentCollisionPanels` (0x08009EE4): the panels of its region (none once it ends). Drawn only.
    if s.highlight then
        me:highlight_collision_panels()
    end
end

-- 0x080CE704: its look, element, range, speed and collision from its variant, on the panel under it. (The game
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
    c:present()
    me:set_lifecycle("update")
    tick(me)
end

flying_shot.kind = new.kind {
    id = "flying-shot",
    pool = "attack",
    state = { variant = "record:flying-shot-variant", range = "u8", highlight = "bool" },
    -- 0x080CE6E0.
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

-- 0x080CE80E: a flying shot of `variant` from `owner` at `pos`, with the damage word; it takes the owner's side and
-- runs later this tick, right after its spawner.
function flying_shot.launch(owner: Object, pos: Vec3, variant: FlyingShotVariant, damage: number): Object?
    local o = battle.spawn(flying_shot.kind, pos)
    if o then
        (o.state :: State).variant = variant
        o:set_damage_word(damage)
        o.alliance = owner.alliance
        o.related1 = owner
    end
    return o
end

return flying_shot
