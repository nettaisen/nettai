--!strict
-- Gregar's summon (the original's actor object #0x30; the Japanese ROMs'
-- 0x080C3E0C in the Falzar ROM, 0x080C566C in the Gregar ROM; the US ROMs
-- point the slot at a placeholder; docs/engine/beast-chips.md): the cyber
-- beast, drawn in two pieces, one panel behind its user. Both pieces
-- appear (60 ticks), flash white (15), rise (30), and stay (180); the
-- second piece leads: it brings the arrival's look and clears the field's
-- obstacles as it appears, and while it stays it roars, breathes flames
-- over the three columns ahead of its user (./flame), and drops a rock
-- every 20 ticks on a panel of the other side's (chips/grndman/rock), then
-- tells the controller it is done. Both then sink (10) and go.

local arrival = require("../../lib/beast-chips/arrival")
local flame = require("./flame")
local regions = require("../../lib/regions")
local rock = require("../grndman/rock")
local targets = require("../../lib/beast-chips/targets")

local summon = {}

export type State = {
    -- Which piece (its Param1): 0, or 1, the piece that leads. Its
    -- animations are the four from 4 × piece.
    piece: number,
    -- The rocks' damage (Param2: the chip's parameter byte, which the
    -- controller passes on).
    rock_damage: number,
    -- The Atk+ / cross bonus (+0x30).
    bonus: number,
    -- The controller, whose flag byte (its Param3) the summon holds: set
    -- while it is out (+0x54).
    controller: Object?,
    -- The last panel a rock was aimed at (+0x60, +0x64).
    last_x: number,
    last_y: number,
    -- Ticks to the next roar (+0x68).
    roar: number,
}

local SPRITE = asset.sprite("gregar-summon")
-- The arrival's look: animation 8 of its sprite.
local ARRIVAL_ANIM = 8

local SOUND_APPEAR = asset.sound("log-in")
local SOUND_FLASH = asset.sound("bug")
local SOUND_ROAR = asset.sound("beast-out-chosen-gregar")
local SOUND_GROWL = asset.sound("roar")

-- Its actions.
local APPEAR, FLASH, RISE, STAY, ATTACK, SINK = 0x0, 0x4, 0x8, 0xC, 0x10, 0x14
-- The actions' ticks.
local APPEAR_TICKS, FLASH_TICKS, RISE_TICKS, STAY_TICKS, SINK_TICKS = 0x3C, 0x0F, 0x1E, 0xB4, 0x0A
-- Every 20 ticks a rock, every 16 a growl; the camera shakes this hard.
local ROCK_EVERY, GROWL_EVERY, SHAKE = 0x14, 0x10, 3
-- The flames: on the panels of region 0x1A around the panel two ahead of
-- it, for 180 ticks.
local FLAMES_AHEAD, FLAME_TICKS = 2, 0xB4
-- The rocks (0x01010105): their panel blinks 5 ticks, they act and
-- animate while dimmed, crack the panel they land on, without a sound.
local ROCK: GroundRockDrop = { warning = 5, while_dimmed = true, cracks = true, silent = true }

-- The piece's animation `n` (0 to 3).
local function anim(me: Object, n: number): number
    return n + (me.state :: State).piece * 4
end

-- A timed action's first tick: its animation, then `ticks` to count.
local function start(me: Object, n: number, ticks: number)
    me.phase_init = 4
    me.anim = anim(me, n)
    me.timer = ticks
end

-- Count down; at 0 the next action.
local function count(me: Object, next: number)
    local t = me.timer - 1
    me.timer = t
    if t <= 0 then
        me:set_action(next)
    end
end

-- `object_isValidPanel`.
local function valid(x: number, y: number): boolean
    return x >= 1 and x <= 6 and y >= 1 and y <= 3
end

-- 0x080C4244: every field object's HP to 0 (the registry's eight slots,
-- stage objects included), which breaks them.
local function clear_field()
    for _, o in field.all_objects() do
        o.hp = 0
    end
end

-- 0x080C4098: flames on the panels of region 0x1A around the panel two
-- ahead of it (the anchor, then the two columns ahead), those on the
-- field, with its damage word and bonus.
local function breathe(me: Object)
    local s = me.state :: State
    local dir = me.facing
    local x0, y0 = me.panel_x + dir * FLAMES_AHEAD, me.panel_y
    local damage = int.u32(bit32.bor(me.damage, bit32.lshift(me.stamina, 16)) + s.bonus)
    for _, p in regions.deep_fan.panels :: { { number } } do
        local x, y = x0 + p[1] * dir, y0 + p[2]
        if valid(x, y) then
            flame.spawn(me, x, y, me.element, damage, FLAME_TICKS, s.bonus)
        end
    end
end

-- 0x080C41BC: a rock on (x, y): its damage the rocks' damage and the
-- bonus, with the summon's counter byte and its damage word's flag bits.
-- (The spawner's Z register holds those flag bits.)
local function drop_rock(me: Object, x: number, y: number)
    local s = me.state :: State
    local flags = bit32.band(me.damage, 0xF000)
    local word = bit32.bor(int.u32(bit32.bor(s.rock_damage, bit32.lshift(me.stamina, 16)) + s.bonus), flags)
    rock.drop(me, x, y, me.element, word, ROCK, flags)
end

-- 0x080C3EBC: shown, with a sound; the leading piece brings the arrival
-- and breaks the field's obstacles; 60 ticks.
local function appear(me: Object)
    if me.phase_init == 0 then
        me.phase_init = 4
        me.visible = true
        me.timer = APPEAR_TICKS
        battle.play_sound(SOUND_APPEAR)
        if (me.state :: State).piece ~= 0 then
            arrival.spawn(me, SPRITE, ARRIVAL_ANIM, APPEAR_TICKS)
            clear_field()
        end
        return
    end
    count(me, FLASH)
end

-- 0x080C3F04: white, with a sound; 15 ticks.
local function flash(me: Object)
    if me.phase_init == 0 then
        start(me, 1, FLASH_TICKS)
        battle.play_sound(SOUND_FLASH)
        me.sprite.white = true
        return
    end
    count(me, RISE)
end

-- 0x080C3F40: its colors back; 30 ticks; then the first piece stays and
-- the second attacks.
local function rise(me: Object)
    if me.phase_init == 0 then
        start(me, 2, RISE_TICKS)
        me.sprite.white = false
        return
    end
    count(me, STAY + (me.state :: State).piece * 4)
end

-- 0x080C3F7A: 180 ticks, the first counting down at once.
local function stay(me: Object)
    if me.phase_init == 0 then
        start(me, 3, STAY_TICKS)
    end
    count(me, SINK)
end

-- 0x080C3FA8: the attack, 180 ticks: a roar and a growl, the flames, the
-- camera shaking; from the first tick a rock every 20 ticks on a fresh
-- panel and a growl every 16.
local function attack(me: Object)
    local s = me.state :: State
    if me.phase_init == 0 then
        start(me, 3, STAY_TICKS)
        me.sprite.white = false
        me.timer2 = 0
        battle.play_sound(SOUND_ROAR)
        battle.play_sound(SOUND_GROWL)
        s.roar = GROWL_EVERY
        breathe(me)
        battle.shake_camera(SHAKE, me.timer)
    end
    local t2 = me.timer2 - 1
    me.timer2 = t2
    if t2 <= 0 then
        me.timer2 = ROCK_EVERY
        local x, y = targets.pick(me, s.last_x, s.last_y)
        if x and y then
            s.last_x, s.last_y = x, y
            drop_rock(me, x, y)
        end
    end
    local r = s.roar - 1
    s.roar = r
    if r <= 0 then
        s.roar = GROWL_EVERY
        battle.play_sound(SOUND_GROWL)
    end
    count(me, SINK)
end

-- 0x080C4030: 10 ticks; then the leading piece clears the controller's
-- flag, and it goes.
local function sink(me: Object)
    local s = me.state :: State
    if me.phase_init == 0 then
        start(me, 2, SINK_TICKS)
        return
    end
    local t = me.timer - 1
    me.timer = t
    if t > 0 then
        return
    end
    if s.piece ~= 0 then
        local c = s.controller
        -- (The original writes the byte wherever the controller was.)
        if c and c.active then
            (c.state :: { gregar_out: boolean }).gregar_out = false
        end
    end
    me:set_lifecycle("destroy")
end

-- 0x080C3E92: by action (0x080C3EA4).
local function tick(me: Object)
    local a = me.action
    if a == APPEAR then
        appear(me)
    elseif a == FLASH then
        flash(me)
    elseif a == RISE then
        rise(me)
    elseif a == STAY then
        stay(me)
    elseif a == ATTACK then
        attack(me)
    elseif a == SINK then
        sink(me)
    else
        error(string.format("Gregar's summon: action %#x reads past its table (0x080C3EA4)", a))
    end
end

-- 0x080C3E30: on its panel, on the ground but a pixel down the field and
-- up for each piece number; its sprite without a shadow, the piece's
-- first animation, facing its way. (The original decompresses the sprite
-- first: the pack's is ready.)
local function init(me: Object)
    local s = me.state :: State
    me:set_coordinates_from_panel()
    local p = me.pos
    local lift = s.piece * 0x1_0000
    me.pos = Vec3.new(p.x, p.y + lift, lift)
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "with_sprite"
    local a = anim(me, 0)
    me.anim, me.anim_loaded = a, a
    sprite:set_animation(a)
    sprite.palette = 0
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    s.last_x, s.last_y = 0, 0
    me:set_lifecycle("update")
end

summon.kind = define.kind {
    id = "gregar/summon",
    pool = "actor",
    state = {
        piece = "u8",
        rock_damage = "u8",
        bonus = "u16",
        controller = "object",
        last_x = "u32",
        last_y = "u32",
        roar = "i32",
    },
    -- 0x080C3E0C: by state (its end `object_freeMemory`), then the sprite,
    -- stepping while dimmed.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
        elseif l == "update" then
            tick(me)
        else
            me:free()
            return
        end
        me:update_sprite_while_dimmed()
    end,
}

-- 0x080C4068: piece `piece` of Gregar for `user` on (x, y), of its side
-- and facing, with `element`, the damage word, the bonus and the rocks'
-- damage, holding `controller`'s flag (set). (Spawned at the registers:
-- the row, the element and the bonus; its init places it.)
function summon.spawn(
    controller: Object,
    user: Object,
    x: number,
    y: number,
    element: number,
    damage: number,
    bonus: number,
    piece: number,
    rock_damage: number
): Object?
    local o = battle.spawn(summon.kind, Vec3.new(y, element, bonus))
    if not o then
        return nil
    end
    o.panel_x, o.panel_y = x, y
    o.element = element
    o.related1 = user
    o.alliance, o.flip = user.alliance, user.flip
    o:set_damage_word(damage)
    local s = o.state :: State
    s.piece, s.rock_damage, s.bonus = piece, rock_damage, bonus
    s.controller = controller
    ;(controller.state :: { gregar_out: boolean }).gregar_out = true
    return o
end

return summon
