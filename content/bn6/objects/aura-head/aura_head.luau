--!strict
-- An aura head (attack object #0x92, `sub_80D7400`): AuraHed's and
-- StreamHd's. It charges along its row at 12 pixels a tick from where it
-- was fired, hitting each panel it enters (once per panel: after a hit it
-- is harmless until it leaves that panel), and on its first hit shakes the
-- screen and goes on for a few ticks more. Then it blinks out over 30
-- ticks. Its first parameter is its palette, its second its speed, its
-- third its hit modifier (1 or 3), and with its fourth it runs to the far
-- edge of the field rather than 3 panels.

local aura_head = {}

export type State = {
    -- The panel (x | y << 8) it last hit on, where it keeps its region
    -- off (`+0x60`), and whether it has hit anything yet (`+0x64`).
    hit_panel: number,
    has_hit: boolean,
}

aura_head.state = { hit_panel = "u32", has_hit = "bool" }

-- `sub_8011504`'s words: the sprite (04-03, animation 4, a ground
-- shadow), and the collision (self type 6, target 5, hit modifier 3 or
-- 1 by the third parameter).
local SPRITE, ANIM = "04-03", 4
-- The speeds by second parameter (`byte_80D74D4`), 16.16 per tick.
local SPEEDS = { [0] = 0xC_0000, 0xC_0000 }
-- A panel's width, 16.16.
local PANEL = 0x28_0000
-- Panels it charges across without its fourth parameter.
local SHORT = 3
local FADE = 0x1E
local DUST = 0x14

-- `sub_80D75D4`: a head for `user` on (x, y), with the damage word
-- `damage` and parameters `params`. It starts 12 pixels down (the
-- spawner's Z register).
function aura_head.spawn(user: Object, x: number, y: number, element: number, params: { number }, damage: number): Object?
    local o = battle.spawn_kind("aura-head", Vec3.new(y, element, -0xC_0000), params)
    if o then
        o.panel_x, o.panel_y = x, y
        o.element = element
        o.damage = damage
        o.stamina = bit32.rshift(damage, 16)
        o.alliance, o.flip = user.alliance, user.flip
        o.related1 = user
    end
    return o
end

-- `sub_80D7578` (action 0): charge; out of time, fade.
local function charge(me: Object)
    local p = me.pos
    me.pos = Vec3.new(p.x + me.vel.x, p.y, p.z)
    me:set_panel_from_coordinates()
    me:update_collision_panels()
    local t = me.timer - 1
    me.timer = t
    if t ~= 0 then
        return
    end
    me.timer = FADE
    me.collision.region = 0
    me.action = 4
    me.phase = 0
    me.phase_init = 0
end

-- `sub_80D75A4` (action 4): blink out; gone off the field.
local function fade(me: Object)
    if not field.valid(me.panel_x, me.panel_y) then
        me:set_lifecycle("destroy")
        return
    end
    me.visible = bit32.btest(me.timer, 2)
    local t = me.timer - 1
    me.timer = t
    if t == 0 then
        me:set_lifecycle("destroy")
    end
end

-- `sub_80D74DC`: resolve its hits; the first shakes the screen and cuts
-- its run short.
local function tick(me: Object)
    local c = me.collision
    local s = me.state :: State
    c:remove()
    me:hit_spark()
    if c.hit_flags ~= 0 then
        s.hit_panel = me.panel_x + me.panel_y * 0x100
        c.region = 0
        if not s.has_hit then
            me.timer = int.tdiv(PANEL, me.vel.y) + 2
            me.timer2 = 0
            battle.shake_camera(1, 0x1E)
            s.has_hit = true
        end
    end
    local a = me.action
    if a == 0 then
        charge(me)
    elseif a == 4 then
        fade(me)
    else
        error(string.format("aura head action %#x reads past its table", a))
    end
    local highlight = true
    if c.region == 0 and me.action ~= 4 and me.lifecycle ~= "destroy" then
        if me.panel_x + me.panel_y * 0x100 == s.hit_panel then
            highlight = false
        else
            c.region = 1
        end
    end
    if highlight then
        field.highlight(me.panel_x, me.panel_y)
    end
    me:update_sprite()
    c:present()
end

-- Off the field or without collision: a puff of dust 16 pixels up, and
-- gone.
local function fizzle(me: Object)
    local p = me.pos
    battle.effect(Vec3.new(p.x, p.y, p.z + 0x10_0000), DUST)
    me:free()
end

-- `sub_80D7420`.
local function init(me: Object)
    me:set_coordinates_from_panel()
    if not field.valid(me.panel_x, me.panel_y) then
        return fizzle(me)
    end
    -- `sub_8011504`.
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite:set_animation(0)
    sprite.shadow = "ground"
    me.anim = ANIM
    me.anim_loaded = 0xFF
    if not me:create_collision() then
        return fizzle(me)
    end
    me:setup_collision(6, 5, if me:param(3) ~= 0 then 3 else 1)
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    me.visible = true
    sprite.palette = me:param(1)
    local speed = SPEEDS[me:param(2)]
    if speed == nil then
        error(string.format("aura head speed %d reads past byte_80D74D4", me:param(2)))
    end
    local vx = speed * me.facing
    me.vel = Vec3.new(vx, speed, me.vel.z)
    local p = me.pos
    me.pos = Vec3.new(p.x + vx * 2, p.y, p.z)
    me.collision.hit_effect = 0x0A
    local panels = SHORT
    if me:param(4) ~= 0 then
        -- To the far edge: column 7 for side 0, column 0 for side 1.
        panels = math.abs(bit32.bxor(me.alliance, 1) * 7 - me.panel_x)
    end
    me.timer = int.tdiv(PANEL * panels - speed - speed, speed) + 1
    me.collision:present()
    battle.play_sound(0x120)
    me.lifecycle = "update"
    tick(me)
end

-- `sub_80D7400`.
function aura_head.update(me: Object)
    local l = me.lifecycle
    if l == "init" then
        init(me)
    elseif l == "update" then
        tick(me)
    else
        me:destroy()
    end
end

return aura_head
