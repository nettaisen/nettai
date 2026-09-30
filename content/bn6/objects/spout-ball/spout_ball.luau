--!strict
-- SpoutMan's water ball (attack object #0x22, `sub_80C853C`): thrown from
-- 24 pixels ahead of him at a height of 16, it flies 4 pixels a tick
-- forward for 14 ticks, falling to the ground, onto the panel 2 columns
-- ahead; if that panel is solid it splashes there and on the panel beyond
-- (if that is solid too; objects/spout-splash), the first splash cracking
-- its panel when it goes. SpoutMan's (Param1 4) act and step their sprite
-- while dimmed; others (his navi AI's) stand still while dimmed. See
-- docs/engine/chips.md §3.6.11.

local spout_splash = require("../spout-splash/spout_splash")

local spout_ball = {}

local SPRITE = "0C-23"

-- Where it starts, ahead of its panel and above it; its speed forward
-- (16.16).
local AHEAD = 0x18_0000
local HEIGHT = 0x10_0000
local SPEED = 0x4_0000

-- `sub_80C86B0`: a ball from (x, y) with `element`, the damage word and
-- `params` {Param1}. (Its position is the spawner's registers, all of
-- which its init overwrites.)
function spout_ball.spawn(owner: Object, x: number, y: number, element: number, params: { number }, damage: number): Object?
    local o = battle.spawn_kind("spout-ball", Vec3.zero, params)
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

-- `sub_80C856C`: ahead of its panel, in the air.
local function init(me: Object)
    me:set_coordinates_from_panel()
    local p = me.pos
    me.pos = Vec3.new(p.x + me.facing * AHEAD, p.y, HEIGHT)
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "with_sprite"
    me.visible = true
    me.anim = 1
    me.anim_loaded = 1
    sprite:set_animation(1)
    sprite.palette = 0
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    me:set_lifecycle("update")
end

-- `sub_80C85F4`: fly (the flight's ticks and fall come from two BIOS
-- divisions: (0x50_0000 - AHEAD) / SPEED ticks, HEIGHT / ticks a tick);
-- on landing, splash on a solid panel, else go.
local function fly(me: Object)
    if me.phase == 0 then
        me.phase = 4
        local ticks = (0x50_0000 - AHEAD) // SPEED
        me.vel = Vec3.new(me.facing * SPEED, me.vel.y, HEIGHT // ticks)
        me.timer = ticks
        return
    end
    local p, v = me.pos, me.vel
    me.pos = Vec3.new(p.x + v.x, p.y, p.z - v.z)
    me:set_panel_from_coordinates()
    local left = me.timer - 1
    me.timer = left
    if left > 0 then
        return
    end
    if field.solid(me.panel_x, me.panel_y) then
        me:set_action(4)
    else
        me:set_lifecycle("destroy")
    end
end

-- `sub_80C8660`: splash here (Param2 1: this one cracks its panel) and on
-- the next panel if it is solid; then go.
local function splash(me: Object)
    battle.play_sound(0x11D)
    local damage = bit32.bor(me.damage, bit32.lshift(me.stamina, 16))
    local x, y = me.panel_x, me.panel_y
    -- r3 (1 << 8, then 0 << 8) becomes each splash's Z.
    spout_splash.spawn(me, x, y, me.element, { me:param(1), 1 }, damage, 0x100)
    local next_x = x + me.facing
    if field.solid(next_x, y) then
        spout_splash.spawn(me, next_x, y, me.element, { me:param(1), 0 }, damage, 0)
    end
    me:set_lifecycle("destroy")
end

local ACTIONS = { [0] = fly, [4] = splash }

-- `sub_80C85C0`: gone once the battle is over; outside a dimming unless it
-- is SpoutMan's.
local function tick(me: Object)
    if battle.over() then
        me:set_lifecycle("destroy")
        return
    end
    if me:param(1) ~= 4 and battle.dimmed() then
        return
    end
    local action = ACTIONS[me.action]
    if not action then
        error(string.format("SpoutMan's ball action %#x reads past its table (off_80C85EC)", me.action))
    end
    action(me)
end

-- `sub_80C853C`.
function spout_ball.update(me: Object)
    local l = me.lifecycle
    if l == "init" then
        init(me)
    elseif l == "update" then
        tick(me)
    else
        me:destroy()
    end
    if me:param(1) == 4 then
        me:update_sprite_while_dimmed()
    else
        me:update_sprite_while_paused()
    end
end

return spout_ball
