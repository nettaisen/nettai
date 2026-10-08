--!strict
-- SandRing's ring (attack object 0x6D, 0x080DAFBC: EXE4's own), as SandRing's parameter (3) makes it: shown, the ring
-- 0c-3a (its animation 0), 26 pixels up (the spawner's height, which placing it on its panel keeps), flying toward the
-- other side 10 pixels a tick, an attack hitting navis (hit modifier 1) with its element's spark, its sound (0x8D) as
-- it sets off. It goes at its first hit, turning the panel it hit on to sand (type 10, the pitfall: 0x08009DC4), at the
-- battle's end, or off the field.
--
-- The kind's other parameters (0 to 2: the ring 0c-37 at 10, 14 or 18 pixels a tick, with the status effects 0x10 to
-- 0x12, hit modifier 0 and sound 0x91, the rows at 0x080DAFB0; no sand) no chip or weapon routine passes: only action
-- 0x47 spawns the ring (0x080DB0CC), only SandRing (and weapon routine 0x44, which loads it) uses that action, and its
-- parameter is 3.

local collision = require("../../rules/collision")
local sparks = require("../../lib/sparks")

local ring = {}

local SPRITE = asset.sprite("sand-ring")
local SOUND = asset.sound("spreader")
local SPEED = 10
local HIT_MOD = 1
local SAND: PanelType = "pitfall"

-- Gone: off its panels, hidden, then freed.
local function vanish(me: Object)
    me.collision:set_region(nil)
    me.visible = false
    me:set_lifecycle("destroy")
end

-- 0x080DAFDC.
local function init(me: Object)
    me.visible = true
    local sprite = me.sprite
    sprite:load(SPRITE)
    me.anim, me.anim_loaded = 0, 0
    sprite:set_animation(0)
    sprite:step()
    sprite.shadow = "with_sprite"
    me:set_coordinates_from_panel()
    me.vel = Vec3.new(me.facing * SPEED * 0x10000, me.vel.y, me.vel.z)
    if not me:create_collision() then
        me:free()
        return
    end
    me:setup_collision(collision.attack, collision.hits_navis, HIT_MOD)
    -- (0x08012F7A: the hit effect is its element's.)
    me.collision:set_hit_effect(sparks.of_element(me.element))
    me.collision:present()
    battle.play_sound(SOUND)
    me:set_lifecycle("update")
end

-- 0x080DB06C: its hits resolve and spark; a hit turns its panel to sand and ends it; else on along its row.
local function fly(me: Object)
    local c = me.collision
    c:remove()
    me:hit_spark()
    if c.hit_flags ~= 0 then
        field.set_type(me.panel_x, me.panel_y, SAND)
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
    id = "sandring/ring",
    pool = "attack",
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

-- 0x080DB0CC: a ring of `owner`'s on (x, y) at height `z`, of `element` and the damage word. (Its position is the
-- spawner's registers until its init places it: Y, the element and the height.)
function ring.spawn(owner: Object, x: number, y: number, z: number, element: number, damage: number)
    local o = battle.spawn(ring.kind, Vec3.new(y, element, z))
    if o then
        o.panel_x, o.panel_y = x, y
        o.element = element
        o:set_damage_word(damage)
        o.alliance = owner.alliance
    end
end

return ring
