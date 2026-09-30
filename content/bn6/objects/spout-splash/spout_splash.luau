--!strict
-- A splash of SpoutMan's water ball (attack object #0x23, `loc_80C86D8`):
-- it stands on its panel hitting it (its region gone once it has hit
-- something) for 30 ticks; when it goes, a solid panel under it cracks if
-- Param1 >= 2 and Param2 is set (the ball's own panel). SpoutMan's (Param1
-- 4) act and step their sprite while dimmed; others (his navi AI's) stand
-- still, collision on, while dimmed. See docs/engine/chips.md §3.6.11.

local spout_splash = {}

local SPRITE = "0C-1A"

-- `sub_80C8806`: a splash on (x, y) from `owner` with `element`, the damage
-- word and `params`. `z` is the spawner's register that becomes its Z,
-- whose fraction it keeps.
function spout_splash.spawn(
    owner: Object,
    x: number,
    y: number,
    element: number,
    params: { number },
    damage: number,
    z: number
): Object?
    local o = battle.spawn_kind("spout-splash", Vec3.new(y, element, z), params)
    if o then
        o.panel_x, o.panel_y = x, y
        o.element = element
        o.damage = damage
        o.stamina = bit32.rshift(damage, 16)
        o.alliance, o.flip = owner.alliance, owner.flip
        o.run_while_dimmed = true
    end
    return o
end

-- `sub_80C8708`: on its panel on the ground, collision on.
local function init(me: Object)
    me:set_coordinates_from_panel()
    local p = me.pos
    me.pos = Vec3.new(p.x, p.y, bit32.band(p.z, 0xFFFF))
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "with_sprite"
    me.visible = true
    me.anim = 0
    me.anim_loaded = 0
    sprite:set_animation(0)
    sprite.palette = 0
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    if not me:create_collision() then
        me:free()
        return
    end
    me:setup_collision(0x0A, 5, 3)
    me.collision.hit_effect = 2
    me.collision:present()
    me:set_lifecycle("update")
end

-- `sub_80C87B8`: 30 ticks.
local function stand(me: Object)
    if me.phase == 0 then
        me.phase = 4
        me.timer = 0x1E
        return
    end
    local left = me.timer - 1
    me.timer = left
    if left > 0 then
        return
    end
    me.collision.region = 0
    me:set_lifecycle("destroy")
end

-- `sub_80C876C`: resolve the hits (its region goes once it hit), act
-- (outside a dimming unless it is SpoutMan's), and register again.
local function tick(me: Object)
    local c = me.collision
    c:remove()
    me:hit_spark()
    if battle.over() then
        c.region = 0
        me:set_lifecycle("destroy")
        return
    end
    if c.hit_flags ~= 0 then
        c.region = 0
    end
    if me:param(1) == 4 or not battle.dimmed() then
        if me.action ~= 0 then
            error(string.format("SpoutMan's splash action %#x reads past its table (off_80C87B4)", me.action))
        end
        stand(me)
    end
    c:present()
end

-- `sub_80C87DC`: crack a solid panel (Param1 >= 2, Param2 set), and go.
local function finish(me: Object)
    if field.solid(me.panel_x, me.panel_y) and me:param(1) >= 2 and me:param(2) ~= 0 then
        field.crack(me.panel_x, me.panel_y)
    end
    me:destroy()
end

-- `loc_80C86D8`.
function spout_splash.update(me: Object)
    local l = me.lifecycle
    if l == "init" then
        init(me)
    elseif l == "update" then
        tick(me)
    else
        finish(me)
    end
    if me:param(1) == 4 then
        me:update_sprite_while_dimmed()
    else
        me:update_sprite_while_paused()
    end
end

return spout_splash
