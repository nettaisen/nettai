--!strict
-- A pillar of fire or lightning on a panel (attack object #0x61,
-- `sub_80D0610`): HeatCross Beast's and ElecCross Beast's charged chips
-- (actions 0x35 and 0x3C, Param1 0 and 1) and some viruses' (Param1 2 to
-- 5). It hits its panel every tick until it hits something, for Param2
-- ticks; once its owner's action is over, only Param3 more (the flames die
-- down, animation 1, when that many are left). Param1 1 (lightning)
-- paralyzes and cracks its panel; 4 and 5 stand back by Param4 * 7 pixels.

local element_pillar = {}

-- The sprite by Param1: flames, or (Param1 1 and past the table) lightning.
local FLAMES, LIGHTNING = "0c-1c", "10-32"

-- The owner's action it lasts through, by Param1 (`byte_80D0794`).
local OWNER_ACTIONS = { [0] = 0x35, [1] = 0x3C, [2] = 0x0E, [3] = 0x0A, [4] = 0x0D, [5] = 0x04 }

-- Param1 values that are flames (the others are lightning, which
-- paralyzes and cracks its panel).
local function is_flame(kind: number): boolean
    return kind == 0 or (kind >= 2 and kind <= 5)
end

-- Collision: what it is and hits, the hit spark, lightning's paralysis.
local SELF_TYPE, TARGET_TYPE = 4, 5
local PARALYZE = 0x12

-- `sub_80D07A0`: a pillar from `owner` on (x, y). Its position is the
-- registers (y, the element, `z`: 0 unless given, Darkness's 6 pixels)
-- until its init places it, keeping Z.
function element_pillar.spawn(owner: Object, x: number, y: number, element: number, params: { number }, damage: number, z: number?): Object?
    local o = battle.spawn_kind("element-pillar", Vec3.new(y, element, z or 0), params)
    if o then
        o.panel_x, o.panel_y = x, y
        o.element = element
        o.alliance, o.flip = owner.alliance, owner.flip
        o.damage = damage
        o.stamina = bit32.rshift(damage, 16)
        o.related1 = owner
    end
    return o
end

-- `sub_80ED94E` / `sub_80EDFEA` (the Beast charged chips'): a pillar on each
-- panel of region `region` from the panel in front of `owner`, facing its
-- way, that is on the field, with the attack's element and damage word.
function element_pillar.spawn_region(owner: Object, region: number, params: { number })
    local shape = data.regions[region]
    if shape == nil then
        error(string.format("region %#x is not in the content", region))
    end
    local front = owner.facing
    local x0, y0 = owner.panel_x + front, owner.panel_y
    local damage = int.u32(owner.attack_damage + bit32.lshift(owner.hit_param, 16))
    for _, offset in shape do
        local x, y = x0 + offset[1] * front, y0 + offset[2]
        if x >= 1 and x <= 6 and y >= 1 and y <= 3 then
            element_pillar.spawn(owner, x, y, owner.attack_element, params, damage)
        end
    end
end

-- `sub_80D0766`: once the owner's action is over, Param3 more ticks; the
-- flames die down with Param3 left.
local function watch_owner(me: Object)
    if me.action == 0 then
        local kind = me:param(1)
        local action = OWNER_ACTIONS[kind]
        if action == nil then
            error(string.format("element pillar kind %d reads past sub_80D0766's action table", kind))
        end
        if (me.related1 :: Object).action ~= action then
            me.timer = me:param(3)
            me.action = me:param(3)
        end
    end
    if me.timer == me:param(3) then
        me.anim = 1
    end
end

-- `sub_80D072C`: resolve the hits (a hit takes its region away); then
-- present again, or end after its time.
local function tick(me: Object)
    local c = me.collision
    c:remove()
    me:hit_spark()
    if c.hit_flags ~= 0 then
        c.region = 0
    end
    local t = me.timer - 1
    me.timer = t
    if t <= 0 then
        c.region = 0
        me:set_lifecycle("destroy")
        return
    end
    c:present()
    watch_owner(me)
    field.highlight(me.panel_x, me.panel_y)
end

-- `sub_80D0634`: on its panel (not off the field), raised 2 pixels, with
-- its collision; lightning cracks the panel.
local function init(me: Object)
    if not field.valid(me.panel_x, me.panel_y) then
        me:free()
        return
    end
    local kind = me:param(1)
    local sprite = me.sprite
    sprite:load(if is_flame(kind) then FLAMES else LIGHTNING)
    sprite.shadow = "hidden"
    me.visible = true
    me.anim = 0
    me.anim_loaded = 0
    sprite:set_animation(0)
    sprite.palette = 0
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    me.timer = me:param(2)
    me:set_coordinates_from_panel()
    if kind == 4 or kind == 5 then
        local p = me.pos
        local x = p.x + me.facing * 0x8_0000 - me:param(4) * 0x7_0000 * me.facing
        me.pos = Vec3.new(x, p.y, p.z)
    end
    local p = me.pos
    me.pos = Vec3.new(p.x, p.y + 0x2_0000, p.z + 0x2_0000)
    if not me:create_collision() then
        me:free()
        return
    end
    local lightning = kind == 1
    me:setup_collision(SELF_TYPE, TARGET_TYPE, if lightning then 1 else 3)
    me.collision.hit_effect = if lightning then 3 else 1
    if lightning then
        me.collision.status_base = PARALYZE
    end
    me.collision:present()
    if not is_flame(kind) then
        field.crack(me.panel_x, me.panel_y)
    end
    me:set_lifecycle("update")
    tick(me)
end

-- `sub_80D0610`.
function element_pillar.update(me: Object)
    local l = me.lifecycle
    if l == "init" then
        init(me)
    elseif l == "update" then
        tick(me)
    else
        me:destroy()
    end
    me:update_sprite()
end

return element_pillar
