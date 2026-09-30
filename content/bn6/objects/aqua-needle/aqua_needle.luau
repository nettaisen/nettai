--!strict
-- An aqua needle (attack object #0x50, `sub_80CE530`): AquaNdl's. It
-- flies down onto its panel in 12 ticks, hits there for one tick, and
-- blinks out over 12 more (cracking the panel first when its second
-- parameter says so). Its first parameter picks its palette (times 4).
-- It hangs still while the battle is dimmed, and splashes (effect 0x14)
-- when the battle ends under it.

local aqua_needle = {}

local SPRITE = "10-17"
-- Speed along X and down, 16.16 per tick, for 12 ticks; it lands 4
-- pixels up.
local SPEED, FLIGHT, LANDED_Z = 0xC_0000, 12, 0x4_0000
-- Ticks it blinks out over once landed.
local FADE = 0x0C
local SPLASH = 0x14
local CRACKED = 0x40

-- `sub_80CE6E6`: a needle for `from`'s side onto (x, y), with the damage
-- word `damage` and parameters `params`. Its position is the spawner's
-- registers (y, the element, 0) until its init places it.
function aqua_needle.spawn(from: Object, x: number, y: number, element: number, params: { number }, damage: number): Object?
    local o = battle.spawn_kind("aqua-needle", Vec3.new(y, element, 0), params)
    if o then
        o.panel_x, o.panel_y = x, y
        o.element = element
        o.alliance, o.flip = from.alliance, from.flip
        o.damage = damage
        o.stamina = bit32.rshift(damage, 16)
    end
    return o
end

-- `sub_8109794`: crack (x, y) unless it is already cracked.
local function crack(x: number, y: number)
    if not bit32.btest(field.flags(x, y), CRACKED) then
        field.crack(x, y)
    end
end

-- `sub_80CE64C` (action 0): fly down; on landing, hit its panel (region
-- on for the next resolution). Its panel is highlighted every other 4
-- ticks.
local function fly(me: Object)
    local p = me.pos
    me.pos = Vec3.new(p.x + me.vel.x, p.y, p.z - me.vel.z)
    local t = me.timer - 1
    me.timer = t
    if t < 0 then
        local q = me.pos
        me.pos = Vec3.new(q.x, q.y, LANDED_Z)
        me:set_coordinates_from_panel()
        me:update_collision_panels()
        me.collision.region = 1
        me.action = 4
        me.phase = 0
        me.phase_init = 0
    end
    if bit32.btest(me.timer, 4) then
        field.highlight(me.panel_x, me.panel_y)
    end
end

-- `sub_80CE690` (action 4): its hit is over; blink out, then go.
local function fade(me: Object)
    if me.phase == 0 then
        me.phase = 1
        me.collision.region = 0
        me.timer = FADE
        battle.play_sound(0x1D0)
        if me:param(2) ~= 0 then
            crack(me.panel_x, me.panel_y)
        end
    end
    if me.timer <= 8 then
        me.visible = bit32.btest(me.timer, 2)
    end
    local t = me.timer - 1
    me.timer = t
    if t < 0 then
        me:set_lifecycle("destroy")
    end
end

-- Gone: hidden, its region cleared.
local function finish(me: Object)
    me.visible = false
    me.collision.region = 0
    me:set_lifecycle("destroy")
end

-- `sub_80CE5F0`: resolve its hits; still while dimmed; gone once it hit
-- something, splashing if the battle is over.
local function tick(me: Object)
    local c = me.collision
    c:remove()
    me:hit_spark()
    if battle.over() then
        battle.effect(me.pos, SPLASH)
        finish(me)
        return
    end
    if not battle.dimmed() then
        if c.hit_flags ~= 0 then
            finish(me)
            return
        end
        local a = me.action
        if a == 0 then
            fly(me)
        elseif a == 4 then
            fade(me)
        else
            error(string.format("aqua needle action %#x reads past its table", a))
        end
    end
    c:present()
end

-- `sub_80CE554`: 12 ticks of flight up and back from its panel.
local function init(me: Object)
    if not field.valid(me.panel_x, me.panel_y) then
        me:free()
        return
    end
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "with_sprite"
    me.visible = true
    sprite.palette = me:param(1) * 4
    me:set_coordinates_from_panel()
    local vx = me.facing * SPEED
    me.timer = FLIGHT
    local p = me.pos
    me.vel = Vec3.new(vx, me.vel.y, SPEED)
    me.pos = Vec3.new(p.x - vx * FLIGHT, p.y, SPEED * FLIGHT + LANDED_Z)
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    if not me:create_collision() then
        me:free()
        return
    end
    me:setup_collision(0x0A, 5, 3)
    local c = me.collision
    c.hit_effect = 2
    c.region = 0
    c:present()
    me.anim = 4
    me.anim_loaded = 4
    sprite:set_animation(4)
    battle.play_sound(0xB3)
    me:set_lifecycle("update")
    tick(me)
end

-- `sub_80CE530`.
function aqua_needle.update(me: Object)
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

return aqua_needle
