--!strict
-- TopMan, the navi of EXE4's TopMan chips (EXE4's actor object #0x0E, 0x080B8520: EXE4's own), and each chip's
-- controller, which brings him (effect object #0x0B, 0x080E2B54: lib/navi_chips' short course). He appears on his
-- user's panel (and goes if it isn't solid, 31 ticks later), setting four tops (./top) on the solid, empty panels two
-- and four columns ahead of him, a row up and down; spins up (31 ticks), leaps over the field toward the other side
-- and lands with a shake, a breaking hit on his panel each 13 ticks, three times (unless his panel isn't solid); and
-- goes in a puff.

local collision = require("../../rules/collision")
local navi_chips = require("../../lib/navi_chips")
local regions = require("@exelib/regions")
local sparks = require("../../lib/sparks")
local top = require("./top")

local topman = {}

export type State = {
    -- The controller waiting for him (the flag it gave him, his +0x4C).
    controller: Object?,
    -- His chip's palette.
    palette: number,
}

local SPRITE = asset.sprite("topman")
local SOUND_APPEAR = asset.sound("appear")
local SOUND_SPIN = asset.sound("top-spin")
local SOUND_LEAP = asset.sound("top-leap")
local SOUND_LAND = asset.sound("metal-punch")
-- The puff he goes in (effect row 0x13).
local PUFF = new.effect { sprite = asset.sprite("puff"), anim = 0, palette = 0 }
local SOLID = 0x10
-- What a panel his top lands on must have none of: bodies and objects.
local OCCUPIED = 0x0F80_0000
-- The tops' panels from his (0x080B8640): columns ahead (by his side's way), rows.
local TOPS: { { number } } = { { 2, -1 }, { 4, -1 }, { 4, 1 }, { 2, 1 } }
-- His leap: up at 11 pixels a tick, across at 0x1B0AD, falling 0x4F80 a tick.
local LEAP_VZ, LEAP_VX, GRAVITY = 0xB_0000, 0x1_B0AD, 0x4F80

local APPEAR, SPIN_UP, LEAP, LAND = 0x0, 0x4, 0x8, 0xC

-- 0x080B85E8: 31 ticks; then, on a solid panel, his tops (0x080B876C: each on a solid panel with no body or object),
-- and he spins up (animation 4); else he goes.
local function appear(me: Object)
    local left = me.timer - 1
    me.timer = int.u16(left)
    if left >= 0 then
        return
    end
    if not field.check(me.panel_x, me.panel_y, SOLID, 0) then
        me:set_lifecycle("destroy")
        return
    end
    for i, d in TOPS do
        local x, y = me.panel_x + me.facing * d[1], me.panel_y + d[2]
        if field.check(x, y, SOLID, OCCUPIED) then
            top.spawn(me, x, y, i - 1)
        end
    end
    me.anim = 4
    me.timer2 = 4
    me.timer = 0x1E
    me:set_action(SPIN_UP)
end

-- 0x080B8650: his spin's sound 4 ticks in; at 15 left animation 5, then 6 once its frame ends (the count waits for
-- it); 31 ticks, then he leaps with its sound.
local function spin_up(me: Object)
    local t2 = me.timer2 - 1
    me.timer2 = int.u16(t2)
    if t2 == 0 then
        battle.play_sound(SOUND_SPIN)
    end
    if me.anim == 5 then
        if not bit32.btest(me.sprite.frame_flags, 0x80) then
            return
        end
        me.anim = 6
    elseif me.timer == 15 then
        me.anim = 5
    end
    local left = me.timer - 1
    me.timer = int.u16(left)
    if left >= 0 then
        return
    end
    local v = me.vel
    me.vel = Vec3.new(me.facing * LEAP_VX, v.y, LEAP_VZ)
    battle.play_sound(SOUND_LEAP)
    me:set_action(LEAP)
end

-- 0x080B86AC: in the air by his velocity, falling; landed (his height below 0), on the ground, his panel his position's,
-- with a shake and its sound. (Landed, his falling speed is register garbage the game leaves; nothing reads it.)
local function leap(me: Object)
    me.timer = int.u16(me.timer + 1)
    local p, v = me.pos, me.vel
    local x, z = int.i32(p.x + v.x), int.i32(p.z + v.z)
    me.pos = Vec3.new(x, p.y, z)
    if z < 0 then
        me.pos = Vec3.new(x, p.y, 0)
        me:set_panel_from_coordinates()
        battle.shake_camera(1, 10)
        battle.play_sound(SOUND_LAND)
        me:set_action(LAND)
    end
    me.vel = Vec3.new(v.x, v.y, int.i32(v.z - GRAVITY))
end

-- `sub_80C53A6`'s: a breaking hit on his panel (row 21's, the breaking spark, hitting navis), resolving while dimmed.
local function hit(me: Object)
    local box = battle.hitbox(me, {
        panel_x = me.panel_x,
        panel_y = me.panel_y,
        element = 0,
        z = 0,
        region = regions.single,
        hit_effect = sparks.breaking,
        target = collision.hits_navis,
        self_type = collision.thrown_break,
        damage = me.damage,
        stamina = me.stamina,
        hit_mod = 3,
    })
    if box then
        box.run_while_dimmed = true
    end
end

-- 0x080B86F0: a hit on his panel each 13 ticks (or he goes, his panel not solid), three times; then he goes.
local function land(me: Object)
    if me.phase_init == 0 then
        me.phase_init = 1
        hit(me)
        if not field.check(me.panel_x, me.panel_y, SOLID, 0) then
            me:set_lifecycle("destroy")
            return
        end
        me.timer = 12
    end
    local left = me.timer - 1
    me.timer = int.u16(left)
    if left >= 0 then
        return
    end
    local p = me.phase + 1
    me.phase, me.phase_init = p, 0
    if p == 3 then
        me:set_lifecycle("destroy")
    end
end

local ACTIONS: { [number]: (Object) -> () } = { [APPEAR] = appear, [SPIN_UP] = spin_up, [LEAP] = leap, [LAND] = land }

-- 0x080B85C6: his action.
local function tick(me: Object)
    local f = ACTIONS[me.action]
    if not f then
        error(string.format("TopMan's action %#x reads past his table (0x080B85D8)", me.action))
    end
    f(me)
end

-- 0x080B8564: his sprite (08-12) with his shadow, shown, in animation 0, his chip's palette, facing his side's way, on
-- his panel; his sound; 31 ticks to his tops.
local function init(me: Object)
    local s = me.state :: State
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "ground"
    me.visible = true
    me.anim, me.anim_loaded = 0, 0
    sprite:set_animation(0)
    sprite.palette = s.palette
    sprite:set_flip(me.alliance)
    me:set_coordinates_from_panel()
    battle.play_sound(SOUND_APPEAR)
    me.timer = 0x1E
    me:set_lifecycle("update")
    tick(me)
end

topman.kind = new.kind {
    id = "topman/navi",
    pool = "actor",
    state = { controller = "object", palette = "u8" },
    -- 0x080B8520: by his state, then his sprite, while dimmed too.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
        elseif l == "update" then
            tick(me)
        else
            -- 0x080B8544: a puff 16 pixels above him (effect row 0x13); the controller moves on; he is freed.
            local p = me.pos
            battle.effect(Vec3.new(p.x, p.y, p.z + 0x10_0000), PUFF)
            local controller = (me.state :: State).controller
            if controller then
                navi_chip.navi_left(controller)
            end
            me:free()
            return
        end
        me:update_sprite_while_dimmed()
    end,
}

-- 0x080B8748: TopMan for the controller's user, with the damage plus the bonus, in the chip's palette. (His position
-- is the spawner's registers: the row, the element and 0, of which his init keeps the height.)
function topman.spawn(controller: Object, user: Object, damage: number, palette: number): Object?
    local o = battle.spawn(topman.kind, Vec3.new(controller.panel_y, controller.element, 0))
    if not o then
        return nil
    end
    o.panel_x, o.panel_y = controller.panel_x, controller.panel_y
    o.element = controller.element
    o.alliance = controller.alliance
    o:set_damage_word(damage)
    local s = o.state :: State
    s.controller = controller
    s.palette = palette
    return o
end

-- What each TopMan chip states for its own controller: the controller's id (the original's effect object #0x0B for all
-- three: compat maps each to it) and his palette (0x080B8592 by the chip's level: 0, the SP's 3, the DS's 6).
export type ChipSpec = { id: string, palette: number }

-- A TopMan chip's controller: the short course (the user warps out, 11 ticks, TopMan, 21 ticks after he left, the user
-- warps back in).
local function controller(chip: ChipSpec): NaviChipController
    local function spawn(c: Object, user: Object, damage: number): Object?
        return topman.spawn(c, user, damage, chip.palette)
    end
    return navi_chips.controller { id = chip.id, name = "TopMan", spawn = spawn, short = navi_chips.SHORT }
end

return { controller = controller }
