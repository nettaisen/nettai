--!strict
-- DustCross Beast's junk (attack object #0xC5, `sub_80DDC10`): thrown from
-- in front of the navi, it arcs in 10 ticks onto its target panel (Param1:
-- x | y << 4), highlighting it as it goes. Where it lands on something (a
-- panel that is solid, or holds a body or object) it bursts, cracks the
-- panel and hits; then it is gone.

local junk_shot = {}

local SPRITE = "10-56"

-- How far in front of and above the navi it starts, 16.16.
local AHEAD, UP = 0x8_0000, 0x8_0000

-- Its flight's ticks, and how far below the ground its arc aims, 16.16.
local FLIGHT, DEPTH = 0xA, 0xD_0000

-- What it bursts on: the panel's solid bit, bodies, neutral objects and
-- blockers.
local LANDS_ON = 0x0F80_0010

-- The burst (effect 5), and its sound.
local BURST, BURST_SOUND = 5, 0x70

-- The actions, on its action byte.
local FLYING, GONE = 0, 4

-- `sub_80DDDCC`: junk from `owner` on (x, y) with `element` and the damage
-- word `damage`, at the navi's height `z`, for the target panel `target`
-- (x | y << 4). (Its X and Y are the spawner's registers, Y and the
-- element, until its init places it.)
function junk_shot.spawn(owner: Object, x: number, y: number, element: number, damage: number, z: number, target: number): Object?
    local o = battle.spawn_kind("junk-shot", Vec3.new(y, element, z), { target })
    if o then
        o.panel_x, o.panel_y = x, y
        o.element = element
        o.damage = damage
        o.stamina = bit32.rshift(damage, 16)
        o.alliance, o.flip = owner.alliance, owner.flip
    end
    return o
end

-- The target panel in Param1.
local function target(me: Object): (number, number)
    local t = me:param(1)
    return bit32.band(t, 7), bit32.rshift(t, 4)
end

-- `sub_80DDD30`: fly; on landing, burst on what is there.
local function fly(me: Object)
    local left = me.timer - 1
    me.timer = left
    if left ~= 0 then
        local p, v = me.pos, me.vel
        me.pos = Vec3.new(p.x + v.x, p.y + v.y, p.z - v.z)
        if not bit32.btest(me.timer, 4) then
            field.highlight(me.panel_x, me.panel_y)
        end
        return
    end
    me:set_coordinates_from_panel()
    local p = me.pos
    me.pos = Vec3.new(p.x, p.y, 0)
    local x, y = me.panel_x, me.panel_y
    if bit32.btest(field.flags(x, y), LANDS_ON) then
        battle.effect(me.pos, BURST)
        battle.play_sound(BURST_SOUND)
        -- (And a camera shake: presentation only.)
        field.crack(x, y)
        me.collision.region = 1
    end
    me.action = GONE
    me.phase, me.phase_init = 0, 0
    me.visible = false
end

-- `sub_80DDDB8`: gone (its hit resolves first).
local function vanish(me: Object)
    me.visible = false
    me.collision.region = 0
    me:set_lifecycle("destroy")
end

-- `sub_80DDCF4`.
local function tick(me: Object)
    local c = me.collision
    c:remove()
    me:hit_spark()
    local a = me.action
    if a == FLYING then
        fly(me)
    elseif a == GONE then
        vanish(me)
    else
        error(string.format("DustCross Beast's junk action %#x reads past its table (sub_80DDCF4)", a))
    end
    me:update_sprite()
    c:present()
end

-- `sub_80DDC30`: in front of the navi and up, aimed at its target, its
-- hit off until it lands.
local function init(me: Object)
    me.visible = true
    local sprite = me.sprite
    sprite:load(SPRITE)
    me.anim = 2
    me.anim_loaded = 2
    sprite:set_animation(2)
    sprite:step()
    sprite.shadow = "ground"
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    me:set_coordinates_from_panel()
    local p = me.pos
    local x, z = p.x + me.facing * AHEAD, p.z + UP
    me.pos = Vec3.new(x, p.y, z)
    local tx, ty = target(me)
    local cx, cy = field.center(tx, ty)
    me.vel = Vec3.new(int.tdiv(cx - x, FLIGHT), int.tdiv(cy - p.y, FLIGHT), int.tdiv(z + DEPTH, FLIGHT))
    me.timer = FLIGHT
    me.panel_x, me.panel_y = tx, ty
    if not me:create_collision() then
        me:free()
        return
    end
    me:setup_collision(0x15, 5, 3)
    -- sub_801A146: the hit spark of its element.
    me.collision.hit_effect = bit32.band(me.element, 0xF)
    me.collision.region = 0
    me.collision:present()
    battle.play_sound(0xFF)
    me.lifecycle = "update"
end

-- `sub_80DDC10`.
function junk_shot.update(me: Object)
    local l = me.lifecycle
    if l == "init" then
        init(me)
    elseif l == "update" then
        tick(me)
    else
        me:destroy()
    end
end

return junk_shot
