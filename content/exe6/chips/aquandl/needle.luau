--!strict
-- An aqua needle (attack object #0x50, `sub_80CE530`): AquaNdl's. It
-- flies down onto its panel in 12 ticks, hits there for one tick, and
-- blinks out over 12 more (cracking the panel first when it was spawned
-- to). Its palette is the chip's (four times its first parameter). It hangs still while the
-- battle is dimmed, and goes in a puff when the battle ends under it.

local collision = require("../../rules/collision")
local effects = require("../../lib/effects")
local regions = require("@exelib/regions")
local sparks = require("../../lib/sparks")

local needle = {}

export type State = {
    -- Its palette (the chip's: four times the original's first parameter),
    -- and whether it cracks its panel as it goes (its second).
    palette: number,
    cracks: boolean,
}

local SPRITE = asset.sprite("aqua-needle")
local ANIM = 4
local SOUND_FALL = asset.sound("aqua-needle-2")
local SOUND_LAND = asset.sound("aqua-needle")
-- Speed along X and down, 16.16 per tick, for 12 ticks; it lands 4
-- pixels up.
local SPEED, FLIGHT, LANDED_Z = 0xC_0000, 12, 0x4_0000
-- Ticks it blinks out over once landed.
local FADE = 0x0C
local CRACKED = 0x40
local HIT_MOD = 3

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
        me.collision:set_region(regions.single)
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
        me.collision:set_region(nil)
        me.timer = FADE
        battle.play_sound(SOUND_LAND)
        if (me.state :: State).cracks then
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
    me.collision:set_region(nil)
    me:set_lifecycle("destroy")
end

-- `sub_80CE5F0`: resolve its hits; still while dimmed; gone once it hit
-- something, in a puff if the battle is over.
local function tick(me: Object)
    local c = me.collision
    c:remove()
    me:hit_spark()
    if battle.over() then
        battle.effect(me.pos, effects.puff)
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
    sprite.palette = (me.state :: State).palette
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
    me:setup_collision(collision.thrown, collision.hits_navis, HIT_MOD)
    local c = me.collision
    c:set_hit_effect(sparks.aqua)
    c:set_region(nil)
    c:present()
    me.anim = ANIM
    me.anim_loaded = ANIM
    sprite:set_animation(ANIM)
    battle.play_sound(SOUND_FALL)
    me:set_lifecycle("update")
    tick(me)
end

needle.kind = new.kind {
    id = "aquandl/needle",
    pool = "attack",
    state = { palette = "u8", cracks = "bool" },
    -- `sub_80CE530`.
    update = function(me: Object)
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
    end,
}

-- `sub_80CE6E6`: a needle for `from`'s side onto (x, y), in `palette`,
-- cracking its panel if `cracks`, with the damage word `damage`. Its
-- position is the spawner's registers (y, the element, 0) until its init
-- places it.
function needle.spawn(from: Object, x: number, y: number, element: number, palette: number, cracks: boolean, damage: number): Object?
    local o = battle.spawn(needle.kind, Vec3.new(y, element, 0))
    if o then
        o.panel_x, o.panel_y = x, y
        o.element = element
        o.alliance, o.flip = from.alliance, from.flip
        o:set_damage_word(damage)
        local s = o.state :: State
        s.palette = palette
        s.cracks = cracks
    end
    return o
end

return needle
