--!strict
-- A crack shot (attack object #0x33, `sub_80CA544`): CrakShot's dug-up
-- panel. On its panel it breaks the panel and flies off it: up 20 pixels,
-- then forward at 8 pixels a tick, hitting whatever it meets, until it has
-- hit something, crossed its Param1 panel centers or left the field. A
-- panel it can't dig (not solid, or with something on it) only cracks, and
-- the shot never appears. Either way it raises dust (effect 0x30).

local crack_shot = {}

local SPRITE = "0c-33"
-- Up, then forward, 16.16 per tick; how high it flies (whole pixels).
local RISE, SPEED, HEIGHT = 0x8_0000, 0x8_0000, 0x14
-- The dust it raises on its panel.
local DUST = 0x30
-- A panel it can dig: solid (0x10) with nothing on it (bodies, neutral
-- objects, blockers, reservations).
local SOLID, CRACKED, OCCUPIED = 0x10, 0x40, 0x0F880080

-- `sub_80CA686`: a shot for `owner` on (x, y), with the damage word
-- `damage`, flying across at most `centers` panel centers. Its position
-- is the spawner's registers (the panel Y, the element and 0) until its
-- init places it.
function crack_shot.spawn(owner: Object, x: number, y: number, element: number, damage: number, centers: number): Object?
    local o = battle.spawn_kind("crack-shot", Vec3.new(y, element, 0), { centers })
    if o then
        o.panel_x, o.panel_y = x, y
        o.element = element
        o.damage = damage
        o.stamina = bit32.rshift(damage, 16)
        o.alliance, o.flip = owner.alliance, owner.flip
        o.related1 = owner
    end
    return o
end

-- `sub_8109794`: crack (x, y) unless it is already cracked.
local function crack(x: number, y: number)
    if not bit32.btest(field.flags(x, y), CRACKED) then
        field.crack(x, y)
    end
end

-- `sub_800E6E8`: `center` lies past `from`, up to and including `to`.
local function crossed(to: number, from: number, center: number): boolean
    if to > from then
        return center > from and center <= to
    end
    return center > to and center <= from
end

-- Out of the fight: hidden, its region cleared, then gone.
local function finish(me: Object)
    me.visible = false
    me.collision.region = 0
    me:set_lifecycle("destroy")
end

-- `sub_80CA61A`: resolve its hits; rise, then fly.
local function tick(me: Object)
    local c = me.collision
    c:remove()
    me:hit_spark()
    if c.hit_flags ~= 0 then
        finish(me)
    else
        local p = me.pos
        if bit32.rshift(p.z, 16) < HEIGHT then
            me.pos = Vec3.new(p.x, p.y, p.z + me.vel.z)
        else
            local cx = field.center(me.panel_x, me.panel_y)
            local x = int.i32(p.x + me.vel.x)
            me.pos = Vec3.new(x, p.y, p.z)
            local out = false
            if crossed(x, p.x, cx) then
                local left = me:param(1) - 1
                me:set_param(1, left)
                out = left <= 0
            end
            if out then
                finish(me)
            else
                me:set_panel_from_coordinates()
                me:update_collision_panels()
                if not field.valid(me.panel_x, me.panel_y) then
                    finish(me)
                end
            end
        end
    end
    c:present()
end

-- `sub_80CA568`: dig the panel and take off, or only crack it.
local function init(me: Object)
    me:set_coordinates_from_panel()
    battle.effect(me.pos, DUST)
    local x, y = me.panel_x, me.panel_y
    if not field.check(x, y, SOLID, OCCUPIED) then
        crack(x, y)
        me:free()
        return
    end
    field.break_panel(x, y)
    battle.play_sound(0xDA)
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite:set_animation(0)
    sprite.shadow = "ground"
    me.anim = 0
    me.anim_loaded = 0xFF
    sprite.palette = 0
    me.visible = true
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    me.vel = Vec3.new(me.facing * SPEED, me.vel.y, RISE)
    me.timer = 2
    if not me:create_collision() then
        me:free()
        return
    end
    me:setup_collision(4, 5, 1)
    me.collision.hit_effect = 5
    me.collision:present()
    me:set_lifecycle("update")
    tick(me)
end

-- `sub_80CA544`.
function crack_shot.update(me: Object)
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
end

return crack_shot
