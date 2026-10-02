--!strict
-- ChargeCross Beast's wave (the original's attack object #0xC4,
-- `sub_80DDA84`), its charged chip's (./wave): it starts
-- on the navi's panel and rolls forward, 12 pixels a tick, to the edge of
-- the field, leaving an afterimage every 5 ticks. It hits what it meets
-- (a flash where it does), and hits again only on another panel. Gone
-- once the battle is over.

local collision = require("../../../../rules/collision")
local regions = require("../../../../lib/regions")

local charge_wave = {}

export type State = {
    -- ExtraVars: the panel it last hit on (PanelXY: x | y << 8).
    hit_panel: number,
    -- ExtraVars+4: ticks to the next afterimage.
    trail: number,
}

local SPRITE = asset.sprite("charge-wave")

-- Its speed, 16.16.
local SPEED = 0xC_0000

-- The afterimages: every 5 ticks (the first after 10), a pixel lower and
-- nearer the ground, tinted (`0xCA8F`), lasting 20 ticks without a shadow.
local TRAIL_FIRST, TRAIL_EVERY = 0xA, 5
local TRAIL_SHADER, TRAIL_LIFETIME = 0xCA8F, 0x14

-- The flash where it hits.
local HIT_FLASH = define.effect { sprite = asset.sprite("hit-sparks"), anim = 1, palette = 6 }

-- Its hit: an attack at navis, no spark.
local HIT_MOD = 3

-- The actions, on its action byte.
local ROLLING = 0

-- The panel as the game's halfword PanelXY.
local function panel_word(me: Object): number
    return me.panel_x + me.panel_y * 0x100
end

-- `sub_80DDBDE`: an afterimage of the wave.
local function afterimage(me: Object)
    local p = me.pos
    local pos = Vec3.new(p.x, p.y - 0x1_0000, p.z - 0x1_0000)
    local flip = bit32.bxor(me.alliance, me.flip)
    battle.afterimage(me, pos, { sprite = SPRITE, flip = flip, lifetime = TRAIL_LIFETIME, color_shader = TRAIL_SHADER })
end

-- `sub_80DDB98`: roll on.
local function roll(me: Object)
    local p = me.pos
    me.pos = Vec3.new(p.x + me.vel.x, p.y, p.z)
    me:set_panel_from_coordinates()
    me:update_collision_panels()
end

-- `sub_80DDB24`.
local function tick(me: Object)
    local c = me.collision
    c:remove()
    me:hit_spark()
    if battle.over() then
        c:set_region(nil)
        me:set_lifecycle("destroy")
        return
    end
    local s = me.state :: State
    if c.hit_flags ~= 0 then
        c:set_region(nil)
        s.hit_panel = panel_word(me)
        battle.effect(me.pos, HIT_FLASH)
    end
    local a = me.action
    if a ~= ROLLING then
        error(string.format("ChargeCross Beast's wave action %#x reads past its table (sub_80DDB24)", a))
    end
    roll(me)
    -- sub_80DDBCA
    local left = s.trail - 1
    s.trail = left
    if left <= 0 then
        s.trail = TRAIL_EVERY
        afterimage(me)
    end
    if c.region == nil and panel_word(me) ~= s.hit_panel then
        c:set_region(regions.single)
    end
    if field.valid(me.panel_x, me.panel_y) then
        c:present()
        return
    end
    c:set_region(nil)
    me:set_lifecycle("destroy")
end

-- `sub_80DDAA8`: on the navi's panel, rolling forward, with its hit.
local function init(me: Object)
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "with_sprite"
    me.visible = true
    me.anim = 0
    me.anim_loaded = 0
    sprite:set_animation(0)
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    local front = me.facing
    local v = me.vel
    me.vel = Vec3.new(front * SPEED, v.y, v.z)
    me.panel_x -= front
    me:set_coordinates_from_panel()
    if not me:create_collision() then
        me:free()
        return
    end
    me:setup_collision(collision.attack, collision.hits_navis, HIT_MOD)
    me.collision:set_hit_effect(nil)
    me.collision:present()
    sprite.palette = 0
    local s = me.state :: State
    s.trail = TRAIL_FIRST
    me:set_lifecycle("update")
    tick(me)
end

charge_wave.kind = define.kind {
    id = "chargecross-beast/wave",
    pool = "attack",
    state = { hit_panel = "u16", trail = "i32" },
    -- `sub_80DDA84`.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
        elseif l == "update" then
            tick(me)
        else
            me:destroy()
        end
        me:update_sprite()
    end,
}

-- `sub_80DDBAC`: a wave for `owner` from in front of (x, y), with `element`
-- and the damage word `damage`, at height `z`. (Its X and Y are the
-- spawner's registers, Y and the element, until its init places it.)
function charge_wave.spawn(owner: Object, x: number, y: number, element: number, damage: number, z: number): Object?
    local o = battle.spawn(charge_wave.kind, Vec3.new(y, element, z))
    if o then
        o.panel_x, o.panel_y = x, y
        o.element = element
        o:set_damage_word(damage)
        o.alliance, o.flip = owner.alliance, owner.flip
    end
    return o
end

return charge_wave
