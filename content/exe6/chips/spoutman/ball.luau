--!strict
-- SpoutMan's water ball (the original's attack object #0x22,
-- `sub_80C853C`): thrown from 24 pixels ahead of him at a height of 16, it
-- flies 4 pixels a tick forward for 14 ticks, falling to the ground, onto
-- the panel 2 columns ahead; if that panel is solid it splashes there and
-- on the panel beyond (if that is solid too; ./splash), the first splash
-- cracking its panel when it goes (if the ball's splashes crack). The navi
-- chip's act and step their sprite while dimmed; others (his own AI's)
-- stand still while dimmed. What it is thrown as is a `WaterBall`
-- (types.d.luau; the original's Param1: 4 the navi chip's, 2 and up
-- cracking). See docs/engine/chips.md §3.6.11.

local splash = require("./splash")

local ball = {}

export type State = {
    -- It acts, and animates, while the battle is dimmed (Param1 4).
    while_dimmed: boolean,
    -- Its first splash cracks its panel (Param1 2 and up).
    cracks: boolean,
}

local SPRITE = asset.sprite("countdown-bomb")
local SOUND_SPOUT_BALL = asset.sound("spout-ball")

-- Where it starts, ahead of its panel and above it; its speed forward
-- (16.16).
local AHEAD = 0x18_0000
local HEIGHT = 0x10_0000
local SPEED = 0x4_0000

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

-- `sub_80C8660`: splash here (this one cracks its panel, if the ball's
-- do) and on the next panel if it is solid; then go.
local function splash_down(me: Object)
    battle.play_sound(SOUND_SPOUT_BALL)
    local damage = bit32.bor(me.damage, bit32.lshift(me.stamina, 16))
    local x, y = me.panel_x, me.panel_y
    -- r3 (1 << 8, then 0 << 8) becomes each splash's Z.
    local s = me.state :: State
    splash.spawn(me, x, y, me.element, { while_dimmed = s.while_dimmed, cracks = s.cracks }, damage, 0x100)
    local next_x = x + me.facing
    if field.solid(next_x, y) then
        splash.spawn(me, next_x, y, me.element, { while_dimmed = s.while_dimmed }, damage, 0)
    end
    me:set_lifecycle("destroy")
end

local ACTIONS = { [0] = fly, [4] = splash_down }

-- `sub_80C85C0`: gone once the battle is over; outside a dimming unless it
-- is the navi chip's.
local function tick(me: Object)
    if battle.over() then
        me:set_lifecycle("destroy")
        return
    end
    if not (me.state :: State).while_dimmed and battle.dimmed() then
        return
    end
    local action = ACTIONS[me.action]
    if not action then
        error(string.format("SpoutMan's ball action %#x reads past its table (off_80C85EC)", me.action))
    end
    action(me)
end

ball.kind = new.kind {
    id = "spoutman/ball",
    pool = "attack",
    state = { while_dimmed = "bool", cracks = "bool" },
    -- `sub_80C853C`.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
        elseif l == "update" then
            tick(me)
        else
            me:destroy()
        end
        if (me.state :: State).while_dimmed then
            me:update_sprite_while_dimmed()
        else
            me:update_sprite_while_paused()
        end
    end,
}

-- `sub_80C86B0`: a ball from (x, y) with `element`, the damage word and
-- what it is. (Its position is the spawner's registers, all of which its
-- init overwrites.)
function ball.spawn(owner: Object, x: number, y: number, element: number, what: WaterBall, damage: number): Object?
    local o = battle.spawn(ball.kind, Vec3.zero)
    if o then
        local s = o.state :: State
        s.while_dimmed = what.while_dimmed == true
        s.cracks = what.cracks == true
        o.panel_x, o.panel_y = x, y
        o.element = element
        o:set_damage_word(damage)
        o.alliance, o.flip = owner.alliance, owner.flip
        o.run_while_dimmed = true
    end
    return o
end

return ball
