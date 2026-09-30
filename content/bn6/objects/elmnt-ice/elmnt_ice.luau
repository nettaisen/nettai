--!strict
-- ElmntMan's ice (attack object #0x8E, `sub_80D6D80`): a block of ice on a
-- panel, which it turns to ice, hitting what is there (status 0x50) for
-- Param2 ticks, its region gone once it has hit something. His Aqua puts
-- three in the column ahead of him (Param3 1: it acts and steps its
-- sprite while dimmed); his navi AI outside a dimming (Param3 0: it waits
-- one out, its collision off). See docs/engine/chips.md §3.6.7.

local elmnt_ice = {}

local SPRITE = "10-0F"

-- Its height, in whole pixels (a halfword store over Z's whole part).
local HEIGHT = 8

-- `sub_80D6EB0`: ice on (x, y) with `element` and the damage word;
-- `params` {Param1, ticks, while dimmed}. `z` is the spawner's register
-- that becomes its Z, whose fraction the ice keeps.
function elmnt_ice.spawn(
    owner: Object,
    x: number,
    y: number,
    element: number,
    params: { number },
    damage: number,
    z: number
): Object?
    local o = battle.spawn_kind("elmnt-ice", Vec3.new(y, element, z), params)
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

-- `sub_80D6DB0`: on its panel at its height; collision on, its region
-- off until it acts.
local function init(me: Object)
    me:set_coordinates_from_panel()
    local p = me.pos
    me.pos = Vec3.new(p.x, p.y, bit32.bor(bit32.band(p.z, 0xFFFF), HEIGHT * 0x10000))
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "with_sprite"
    me.visible = true
    me.anim = 1
    me.anim_loaded = 1
    sprite:set_animation(1)
    sprite.palette = 0
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    if not me:create_collision() then
        me:free()
        return
    end
    me:setup_collision(4, 5, 3)
    local c = me.collision
    c.status_base = 0x50
    c.hit_effect = 2
    c.region = 0
    c:present()
    me:set_lifecycle("update")
end

-- `sub_80D6E70`: turn its panel to ice and hit it for Param2 ticks.
local function stand(me: Object)
    local c = me.collision
    if me.phase == 0 then
        me.phase = 4
        c.region = 1
        field.set_type(me.panel_x, me.panel_y, "ice")
        me.timer = me:param(2)
        return
    end
    if c.hit_flags ~= 0 then
        c.region = 0
    end
    local left = me.timer - 1
    me.timer = left
    if left > 0 then
        return
    end
    c.region = 0
    me:set_lifecycle("destroy")
end

-- `sub_80D6E1E`: resolve the hits (its region goes once it hit), then act
-- and register again; outside a dimming unless it runs in one.
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
        c.hit_flags = 0
    end
    if me:param(3) == 0 and battle.dimmed() then
        return
    end
    if me.action ~= 0 then
        error(string.format("ElmntMan's ice action %#x reads past its table (off_80D6E6C)", me.action))
    end
    stand(me)
    c:present()
end

-- `sub_80D6D80`.
function elmnt_ice.update(me: Object)
    local l = me.lifecycle
    if l == "init" then
        init(me)
    elseif l == "update" then
        tick(me)
    else
        me:destroy()
    end
    if me:param(3) == 0 then
        me:update_sprite_while_paused()
    else
        me:update_sprite_while_dimmed()
    end
end

return elmnt_ice
