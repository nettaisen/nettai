--!strict
-- A meteor (the original's attack object #0x8D, `sub_80D6BD4`): it falls
-- from high behind its panel onto it in 17 ticks, highlighting the panel;
-- if the panel holds something of the other side's, it bursts there with
-- an explosion and a one-tick hit region. ElmntMan's Fire drops them
-- during his dimming (the original's Param1 1: it acts, steps its sprite
-- and hits while dimmed); his own AI outside one (Param1 0: it waits out a
-- dimming). See docs/engine/chips.md §3.6.7.

local collision = require("../../rules/collision")
local effects = require("../../lib/effects")
local regions = require("@common/regions")
local sparks = require("../../lib/sparks")

local meteor = {}

export type State = {
    -- It acts, animates and hits while the battle is dimmed (Param1).
    while_dimmed: boolean,
}

local SPRITE = asset.sprite("bn6:meteor")
local SOUND_JUSTICE_ONE = asset.sound("bn6:justice-one")

-- Where it starts, this far behind and above its panel; its speed forward
-- and down; ticks to fall (16.16).
local START = 0xC0_0000
local SPEED = 0xB_0000
local FALL_TICKS = 0x11

-- What it bursts on, by side (`byte_80D6D08`): a panel flags word any bit
-- of which will do.
local TARGETS = { [0] = 0x1580_0010, 0x2A80_0010 }

-- `sub_80D6C04`: high above, behind its panel.
local function init(me: Object)
    me:set_coordinates_from_panel()
    local p = me.pos
    me.pos = Vec3.new(p.x - me.facing * START, p.y, START)
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "with_sprite"
    me.visible = true
    me.anim = 0
    me.anim_loaded = 0
    sprite:set_animation(0)
    sprite.palette = 0
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    me:set_lifecycle("update")
end

-- `sub_80D6D3E`: the hit on its panel (`object_spawnCollisionRegion`; with
-- a meteor that runs while dimmed, one that does too, `sub_80C53A6`).
local function hit(me: Object)
    local box = battle.hitbox(me, {
        panel_x = me.panel_x,
        panel_y = me.panel_y,
        element = me.element,
        z = 0,
        region = regions.single,
        hit_effect = sparks.fire,
        target = collision.hits_navis,
        self_type = collision.thrown,
        damage = me.damage,
        stamina = me.stamina,
        hit_mod = 3,
    })
    if box and (me.state :: State).while_dimmed then
        box.run_while_dimmed = true
    end
end

-- `sub_80D6C88`: fall; on landing, burst if the panel holds a target, and
-- go; highlight the panel 4 ticks out of 8 until then.
local function fall(me: Object)
    if me.phase == 0 then
        me.phase = 4
        me.vel = Vec3.new(me.facing * SPEED, 0, SPEED)
        battle.play_sound(SOUND_JUSTICE_ONE)
        me.timer = FALL_TICKS
    else
        local p, v = me.pos, me.vel
        me.pos = Vec3.new(p.x + v.x, p.y, p.z - v.z)
        local left = me.timer - 1
        me.timer = left
        if left <= 0 then
            if bit32.band(field.flags(me.panel_x, me.panel_y), TARGETS[me.alliance]) ~= 0 then
                battle.effect(me.pos, effects.explosion)
                hit(me)
            end
            me:set_lifecycle("destroy")
            return
        end
    end
    local t = me.timer2 + 1
    me.timer2 = t
    if bit32.band(t, 4) == 0 then
        field.highlight(me.panel_x, me.panel_y)
    end
end

-- `sub_80D6C56`: gone once the battle is over; outside a dimming unless
-- it runs in one.
local function tick(me: Object)
    if battle.over() then
        me:set_lifecycle("destroy")
        return
    end
    if not (me.state :: State).while_dimmed and battle.dimmed() then
        return
    end
    if me.action ~= 0 then
        error(string.format("meteor action %#x reads past its table (off_80D6C84)", me.action))
    end
    fall(me)
end

meteor.kind = define.kind {
    id = "bn6:elmntman/meteor",
    pool = "attack",
    state = { while_dimmed = "bool" },
    -- `sub_80D6BD4`.
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

-- `sub_80D6D18`: a meteor from `owner` onto (x, y), with `element` and the
-- damage word. (Its position is the spawner's registers, all of which its
-- init overwrites.)
function meteor.spawn(owner: Object, x: number, y: number, element: number, damage: number, while_dimmed: boolean): Object?
    local o = battle.spawn(meteor.kind, Vec3.zero)
    if o then
        local s = o.state :: State
        s.while_dimmed = while_dimmed
        o.panel_x, o.panel_y = x, y
        o.element = element
        o:set_damage_word(damage)
        o.alliance, o.flip = owner.alliance, owner.flip
        o.run_while_dimmed = true
    end
    return o
end

return meteor
