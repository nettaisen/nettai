--!strict
-- A feather of Falzar's (the original's attack object #0xBC, `sub_80DCB1C`;
-- its routines are in the US ROMs too, but only the Japanese ROMs' Falzar
-- chip spawns it; docs/engine/beast-chips.md): it dives from 192 pixels up
-- and 192 back onto a panel in 16 ticks, the panel blinking under it, and
-- hits there: a spark, the camera shaking. Falzar's controller sends one
-- every 12 ticks (chips/falzar/controller). A feather spawned to act
-- while dimmed (the chip's) goes on regardless of the battle's end and of
-- the dimming, and steps its sprite while dimmed; another stops when the
-- battle is over and holds while dimmed.

local collision = require("../../rules/collision")
local effects = require("../../lib/effects")
local regions = require("@exelib/regions")
local sparks = require("../../lib/sparks")

local feather = {}

export type State = {
    -- It acts while dimmed, and its hit resolves while dimmed (Param2 1).
    while_dimmed: boolean,
}

local SPRITE = asset.sprite("falzar-feather")
local SOUND = asset.sound("justice-one")

-- Where it starts: 192 pixels back and up (16.16).
local START = 0xC0_0000
-- Its speed back to front and down, and the ticks it dives (the
-- original's tables `byte_80DCC4C` and `byte_80DCC60`, by Param1: the four
-- entries are the same).
local SPEED, DIVE_TICKS = 0xC_0000, 0x10
-- Its hit (0x0A050701): on its panel, an impact spark, thrown, with hit
-- modifier 3.
local HIT_MOD = 3
-- The landing's spark (effect 7) and the camera's shake.
local SHAKE, SHAKE_TICKS = 2, 0x0F

-- `sub_80DCC96`: the hit on its panel; one that acts while dimmed resolves
-- while dimmed (`sub_80C53A6`), another not (`object_spawnCollisionRegion`,
-- whose flags then always skip the other spawn).
local function hit(me: Object, s: State)
    local o = battle.hitbox(me, {
        panel_x = me.panel_x,
        panel_y = me.panel_y,
        element = me.element,
        z = 0,
        region = regions.single,
        hit_effect = sparks.impact,
        target = collision.hits_navis,
        self_type = collision.thrown,
        damage = me.damage,
        stamina = me.stamina,
        hit_mod = HIT_MOD,
    })
    if o and s.while_dimmed then
        o.run_while_dimmed = true
    end
end

-- `sub_80DCBCC`: the first tick its sound and speed; then the dive, and
-- at its end the hit, the spark where it is and the camera shaking, and
-- it is gone. Every tick but the last its panel blinks (4 ticks out of 8).
local function dive(me: Object, s: State)
    if me.phase == 0 then
        me.phase = 4
        battle.play_sound(SOUND)
        me.vel = Vec3.new(me.facing * SPEED, me.vel.y, SPEED)
        me.timer = DIVE_TICKS
    else
        local p, v = me.pos, me.vel
        me.pos = Vec3.new(p.x + v.x, p.y, p.z - v.z)
        local t = me.timer - 1
        me.timer = t
        if t <= 0 then
            hit(me, s)
            battle.effect(me.pos, effects.copy_mark)
            battle.shake_camera(SHAKE, SHAKE_TICKS)
            me:set_lifecycle("destroy")
            return
        end
    end
    local t2 = me.timer2 + 1
    me.timer2 = t2
    if not bit32.btest(t2, 4) then
        field.highlight(me.panel_x, me.panel_y)
    end
end

-- `sub_80DCB9C`: one that doesn't act while dimmed stops when the battle
-- is over, and holds while dimmed; then by action (`off_80DCBC8`, one).
local function tick(me: Object)
    local s = me.state :: State
    if not s.while_dimmed then
        if battle.over() then
            me:set_lifecycle("destroy")
            return
        end
        if battle.dimmed() then
            return
        end
    end
    local a = me.action
    if a ~= 0 then
        error(string.format("Falzar's feather: action %#x reads past its table (off_80DCBC8)", a))
    end
    dive(me, s)
end

-- `sub_80DCB4C`: 192 pixels back from its panel and 192 up, drawn without
-- a shadow, facing its way.
local function init(me: Object)
    me:set_coordinates_from_panel()
    local p = me.pos
    me.pos = Vec3.new(p.x - me.facing * START, p.y, START)
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "with_sprite"
    me.visible = true
    me.anim, me.anim_loaded = 0, 0
    sprite:set_animation(0)
    sprite.palette = 0
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    me:set_lifecycle("update")
end

feather.kind = new.kind {
    id = "falzar/feather",
    pool = "attack",
    state = { while_dimmed = "bool" },
    -- `sub_80DCB1C`: by state (its end `object_genericDestroy`), then the
    -- sprite: stepping while dimmed if it acts while dimmed, else while
    -- paused only.
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

-- `sub_80DCC70`: a feather of `owner`'s onto (x, y) with `element` and the
-- damage word, running while dimmed (flag 0x10; its own gate holds it
-- unless it acts while dimmed). (Spawned at the registers: the row, the
-- element and `z`; its init places it.)
function feather.spawn(owner: Object, x: number, y: number, element: number, damage: number, while_dimmed: boolean, z: number): Object?
    local o = battle.spawn(feather.kind, Vec3.new(y, element, z))
    if o then
        o.panel_x, o.panel_y = x, y
        o.element = element
        o:set_damage_word(damage)
        o.alliance, o.flip = owner.alliance, owner.flip
        o.run_while_dimmed = true
        ;(o.state :: State).while_dimmed = while_dimmed
    end
    return o
end

return feather
