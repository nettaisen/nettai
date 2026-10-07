--!strict
-- MokoRus's Molokos (attack object 0x2C, 0x080D235C: EXE4's own), which the spawners' variant 1 puts out
-- (0x080D252A): one on each row, the rows shuffled (0x08000AAA: three swaps of RNG2's draws), the first at once and the
-- next 10 and 20 ticks later. A Moloko (04-11 in its chip's palette, its parameter's twice; turned to its side, its
-- shadow on the ground) waits 140 pixels behind the field's middle, then charges along its row 3 pixels a tick with
-- its sound (0xC7), a dust puff every 10 ticks (effect row 1, jittered by 7), an attack hitting navis (modifier 0x21,
-- the heavy spark) on the panel it is over. It goes at its first hit; at a panel holding an obstacle of the other
-- side's or a hole, in a puff (effect row 0x15, 16 pixels up); past 140 pixels beyond the middle; or as the battle is
-- over (a puff where it is).

local collision = require("../../rules/collision")
local panels = require("@exelib/panels")
local regions = require("@exelib/regions")
local rock = require("../../lib/rock")
local sparks = require("../../lib/sparks")

local moloko = {}

local SPRITE = asset.sprite("moloko")
local SOUND = asset.sound("moloko")
local ANIM = 2
local HIT_MOD = 0x21
-- 140 pixels from the field's middle either way; 3 pixels a tick; a dust puff every 10 ticks.
local REACH, SPEED, DUST_EVERY = 140, 0x3_0000, 10
local PUFF_UP = 0x10_0000
-- What stops it on its panel, by its side (0x080D2504): an obstacle's body bits.
local BLOCKS = { [0] = 0x01800000, [1] = 0x02800000 }
local SOLID = 0x10

local WAIT, CHARGE = 0x0, 0x4

-- Its palette (twice its Param1, the chip's parameter).
type State = { palette: number }

local function gone(me: Object)
    me.collision:set_region(nil)
    me:set_lifecycle("destroy")
end

-- 0x080D244C: its count, then its hit on its panel, the sound, the charge.
local function wait(me: Object)
    local t = me.timer - 1
    me.timer = bit32.band(t, 0xFFFF)
    if t >= 0 then
        return
    end
    me.collision:set_region(regions.single)
    battle.play_sound(SOUND)
    me.action = CHARGE
    me.phase, me.phase_init = 0, 0
end

-- 0x080D246C.
local function charge(me: Object)
    if me.phase_init == 0 then
        me.vel = Vec3.new(me.facing * SPEED, me.vel.y, me.vel.z)
        me.timer = DUST_EVERY
        me.phase_init = 4
    end
    local p = me.pos
    me.pos = Vec3.new(int.i32(p.x + me.vel.x), p.y, p.z)
    me:set_panel_from_coordinates()
    me:update_collision_panels()
    local t = me.timer - 1
    me.timer = bit32.band(t, 0xFFFF)
    if t <= 0 then
        me.timer = DUST_EVERY
        battle.effect(battle.jitter(7, me.pos), rock.effects.splash)
    end
    -- (Its pixel x: the position's high halfword, signed.)
    local x = int.i16(bit32.rshift(me.pos.x, 16))
    if x + REACH < 0 or x + REACH > 2 * REACH then
        gone(me)
        return
    end
    if not field.valid(me.panel_x, me.panel_y) then
        return
    end
    local flags = field.flags(me.panel_x, me.panel_y)
    if not bit32.btest(flags, BLOCKS[me.alliance]) and bit32.btest(flags, SOLID) then
        return
    end
    local q = me.pos
    battle.effect(Vec3.new(q.x, q.y, q.z + PUFF_UP), rock.effects.puff)
    gone(me)
end

-- 0x080D23FE: the hits against it resolve and spark; gone as the battle is over (in a puff) or once it hit; else its
-- action, present.
local function tick(me: Object)
    local c = me.collision
    c:remove()
    me:hit_spark()
    if battle.over() then
        battle.effect(me.pos, rock.effects.puff)
        gone(me)
        return
    end
    if c.hit_flags ~= 0 then
        gone(me)
        return
    end
    local a = me.action
    if a == WAIT then
        wait(me)
    elseif a == CHARGE then
        charge(me)
    else
        error(string.format("a Moloko's action %#x reads past its table", a))
    end
    c:present()
end

-- 0x080D2380: its sprite on its row 140 pixels behind the field's middle (its panel from there), on the ground, in its
-- palette, turned to its side; an attack hitting navis, no region yet; then its first tick.
local function init(me: Object)
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "ground"
    me.visible = true
    me.anim, me.anim_loaded = ANIM, 0xFF
    me:set_coordinates_from_panel()
    local p = me.pos
    me.pos = Vec3.new(-me.facing * REACH * 0x10000, p.y, 0)
    me:set_panel_from_coordinates()
    sprite.palette = (me.state :: State).palette
    sprite:set_flip(me.alliance)
    if not me:create_collision() then
        me:free()
        return
    end
    me:setup_collision(collision.attack, collision.hits_navis, HIT_MOD)
    me.collision:set_hit_effect(sparks.heavy)
    me.collision:set_region(nil)
    me.collision:present()
    me:set_lifecycle("update")
    tick(me)
end

moloko.kind = new.kind {
    id = "mokorus/moloko",
    pool = "attack",
    state = { palette = "u8" },
    -- 0x080D235C: then the sprite.
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

-- 0x080D252A, the spawners' variant 1: a Moloko in `palette` (twice the chip's parameter) on each row, the rows
-- shuffled, each 10 ticks after the one before, with the attack's element and damage word.
function moloko.spawner(palette: number): (user: Object, element: number, damage: number) -> ()
    return function(user: Object, element: number, damage: number)
        local rows = { 1, 2, 3 }
        panels.shuffle(rows, 3, 3)
        for i, y in rows do
            -- 0x080D250C: on (1, y), its position the registers until its init places it.
            local o = battle.spawn(moloko.kind, Vec3.new(y, element, 0))
            if o then
                o.panel_x, o.panel_y = 1, y
                o.element = element
                o.alliance = user.alliance
                o:set_damage_word(damage)
                o.timer = 10 * (i - 1)
                local s = o.state :: State
                s.palette = palette
            end
        end
    end
end

return moloko
