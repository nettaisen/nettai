--!strict
-- EXE4's rings (attack object 0x6D, 0x080DAFBC: EXE4's own), which action 0x47 throws (lib/rings): shown, 26 pixels up
-- (the spawner's height, which placing it on its panel keeps), flying toward the other side at its speed, an attack
-- hitting navis with its element's spark (0x08012F7A), its sound as it sets off. It goes at its first hit (the sand
-- ring turning the panel it hit on to sand: type 10, the pitfall, 0x08009DC4), at the battle's end, or off the field.
-- What it is by its parameter (0x080DAFB0's 3-byte rows: its speed in pixels a tick, its hits' status, its sound;
-- the sheet 0c-3a and hit modifier 1 for row 3, else 0c-37 and 0) each thrower states (`ring.variant`): SandRing's
-- row 3, the zap ring's row 0 (weapon routine 0x04). (Rows 1 and 2, 14 and 18 pixels a tick with paralysis for 120
-- and 150, no routine passes.)

local collision = require("../rules/collision")
local sparks = require("../lib/sparks")

local ring = {}

type State = { variant: RingVariant }

-- A ring's variant.
function ring.variant(v: RingVariant): RingVariant
    return new.record("ring-variant", v)
end

-- Gone: off its panels, hidden, then freed.
local function vanish(me: Object)
    me.collision:set_region(nil)
    me.visible = false
    me:set_lifecycle("destroy")
end

-- 0x080DAFDC.
local function init(me: Object)
    local v = (me.state :: State).variant
    me.visible = true
    local sprite = me.sprite
    sprite:load(v.sprite)
    me.anim, me.anim_loaded = 0, 0
    sprite:set_animation(0)
    sprite:step()
    sprite.shadow = "with_sprite"
    me:set_coordinates_from_panel()
    me.vel = Vec3.new(me.facing * v.speed * 0x10000, me.vel.y, me.vel.z)
    if not me:create_collision() then
        me:free()
        return
    end
    me:setup_collision(collision.attack, collision.hits_navis, v.hit_mod)
    local status = v.status
    if status then
        me.collision.status_base = status
    end
    -- (0x08012F7A: the hit effect is its element's.)
    me.collision:set_hit_effect(sparks.of_element(me.element))
    me.collision:present()
    battle.play_sound(v.sound)
    me:set_lifecycle("update")
end

-- 0x080DB06C: its hits resolve and spark; a hit ends it (the sand ring's leaving sand); else on along its row.
local function fly(me: Object)
    local v = (me.state :: State).variant
    local c = me.collision
    c:remove()
    me:hit_spark()
    if c.hit_flags ~= 0 then
        local sand = v.sand
        if sand then
            field.set_type(me.panel_x, me.panel_y, sand)
        end
        return vanish(me)
    end
    if battle.over() then
        return vanish(me)
    end
    local p = me.pos
    me.pos = Vec3.new(int.i32(p.x + me.vel.x), p.y, p.z)
    me:set_panel_from_coordinates()
    me:update_collision_panels()
    if not field.valid(me.panel_x, me.panel_y) then
        return vanish(me)
    end
    c:present()
    me:update_sprite()
end

ring.kind = new.kind {
    id = "ring",
    pool = "attack",
    state = { variant = "record:ring-variant" },
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
        elseif l == "update" then
            fly(me)
        else
            me:destroy()
        end
    end,
}

-- 0x080DB0CC: a ring of `variant` of `owner`'s on (x, y) at height `z`, of `element` and the damage word. (Its position
-- is the spawner's registers until its init places it: Y, the element and the height.)
function ring.spawn(owner: Object, variant: RingVariant, x: number, y: number, z: number, element: number, damage: number)
    local o = battle.spawn(ring.kind, Vec3.new(y, element, z))
    if o then
        (o.state :: State).variant = variant
        o.panel_x, o.panel_y = x, y
        o.element = element
        o:set_damage_word(damage)
        o.alliance = owner.alliance
    end
end

return ring
