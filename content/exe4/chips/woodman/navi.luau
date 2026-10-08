--!strict
-- WoodMan, the navi of EXE4's WoodMan chips (EXE4's actor object #0x49, 0x080C980C: EXE4's own), and each chip's
-- controller, which brings him (effect object #0x70: lib/navi_chips' short course). He appears on his user's panel and,
-- 21 ticks later (gone if it isn't solid), crouches (animation 4, 6 ticks) and leaps, landing 54 ticks later with a
-- shake and a crash; as his landing ends (animation 10) and 10 ticks on, he raises four or five wood towers (./tower)
-- on the other side's solid panels at random, and 60 ticks later goes. His command, Down, A and B pressed within five
-- ticks of each other while dimmed as he lands, raises a second set.

local navi_chips = require("../../lib/navi_chips")
local panels = require("@exelib/panels")
local tower = require("./tower")

local woodman = {}

type State = {
    -- The controller waiting for him (the flag it gave him, his +0x40).
    controller: Object?,
    -- His chip's palette.
    palette: number,
    -- Ticks left since each of his command's keys was pressed (+0x60 to +0x62: Down, A, B), and his command done
    -- (Param2).
    down: number,
    a: number,
    b: number,
    again: boolean,
}

local SPRITE = asset.sprite("woodman")
local SOUND_APPEAR = asset.sound("appear")
local SOUND_LAND = asset.sound("metal-punch")
-- The panels his towers rise on, by his side (0x080C9A60: the other side's solid panels).
local TOWER_PANELS: { [number]: { number } } = { [0] = { 0x30, 0x00 }, [1] = { 0x10, 0x20 } }
local SOLID = 0x10
-- His leap: up 27 pixels a tick, falling a pixel a tick faster each.
local LEAP_SPEED, GRAVITY = 0x1B_0000, 0x1_0000
-- A key of his command counts this long.
local PRESS_TICKS = 5

local APPEAR, ATTACK = 0x0, 0x4

-- Enter the phase with a timer, or count down; true once it has run out at 0 (`bgt`).
local function countdown(me: Object, ticks: number): boolean
    if me.phase_init == 0 then
        me.phase_init = 1
        me.timer = ticks
    end
    local left = me.timer - 1
    me.timer = int.u16(left)
    return left <= 0
end

-- 0x080C9A94: his side's navi's presses while dimmed (Down, A, B) each counted 5 ticks; all three counting, his
-- command is done; else each counts down.
local function command(me: Object)
    local s = me.state :: State
    local navi = battle.player(me.alliance)
    if navi then
        if navi:dimmed_pressed("down") then
            s.down = PRESS_TICKS
        end
        if navi:dimmed_pressed("a") then
            s.a = PRESS_TICKS
        end
        if navi:dimmed_pressed("b") then
            s.b = PRESS_TICKS
        end
    end
    if s.down * s.a * s.b ~= 0 then
        s.again = true
        return
    end
    s.down, s.a, s.b = math.max(s.down - 1, 0), math.max(s.a - 1, 0), math.max(s.b - 1, 0)
end

-- Action 0 (0x080C98CC): 21 ticks; then, on a solid panel, his attack; else he goes.
local function appear(me: Object)
    local left = me.timer - 1
    me.timer = int.u16(left)
    if left >= 0 then
        return
    end
    if field.check(me.panel_x, me.panel_y, SOLID, 0) then
        me:set_action(ATTACK)
    else
        me:set_lifecycle("destroy")
    end
end

-- 0x080C99CC's set: four or five (a draw's low bit) of the other side's solid panels but his own at random (a
-- shuffle); a tower on each, the last listed first, the first last with its sound.
local function towers(me: Object)
    local count = bit32.band(battle.rng_positive(), 1) + 4
    local p = TOWER_PANELS[me.alliance]
    local list = panels.filtered_except(me, p[1], p[2])
    local n = #list
    if n == 0 then
        return
    end
    panels.shuffle(list, n, n)
    for i = math.min(n, count), 1, -1 do
        tower.spawn(me, panels.x(list[i]), panels.y(list[i]), me.element, i == 1)
    end
end

-- Action 4 (0x080C98F6): phase 0 (0x080C991C) crouching (animation 4), 6 ticks; phase 4 (0x080C9946) the leap
-- (animation 15 at its top), his command read in its last 4 ticks, 54 ticks; landing on the floor with a shake (2, 15
-- ticks), the crash and animation 7; phase 8 (0x080C9994) his command read till the landing ends, then animation 10;
-- phase 0xC (0x080C99AE) 10 ticks; phase 0x10 (0x080C99CC) his towers, 60 ticks; then, his command done, another
-- round from phase 0xC, else he goes.
local function attack(me: Object)
    local s = me.state :: State
    local phase = me.phase
    if phase == 0 then
        if me.phase_init == 0 then
            me.anim = 4
        end
        if countdown(me, 6) then
            me.vel = Vec3.new(me.vel.x, me.vel.y, LEAP_SPEED)
            me.timer = 54
            me.phase, me.phase_init = 4, 0
        end
    elseif phase == 4 then
        local p, v = me.pos, me.vel
        me.pos = Vec3.new(p.x, p.y, int.i32(p.z + v.z))
        local vz = int.i32(v.z - GRAVITY)
        me.vel = Vec3.new(v.x, v.y, vz)
        if vz == 0 then
            me.anim = 15
        end
        if me.timer < 4 then
            command(me)
        end
        local left = me.timer - 1
        me.timer = int.u16(left)
        if left > 0 then
            return
        end
        p = me.pos
        me.pos = Vec3.new(p.x, p.y, 0)
        battle.shake_camera(2, 15)
        battle.play_sound(SOUND_LAND)
        me.anim = 7
        me.phase, me.phase_init = 8, 0
    elseif phase == 8 then
        command(me)
        if bit32.btest(me.sprite.frame_flags, 0x80) then
            me.anim = 10
            me.phase, me.phase_init = 0xC, 0
        end
    elseif phase == 0xC then
        if countdown(me, 10) then
            me.phase, me.phase_init = 0x10, 0
        end
    elseif phase == 0x10 then
        if me.phase_init == 0 then
            me.phase_init = 1
            me.timer = 60
            towers(me)
        end
        local left = me.timer - 1
        me.timer = int.u16(left)
        if left > 0 then
            return
        end
        if s.again then
            s.again = false
            me.phase, me.phase_init = 0xC, 0
        else
            me:set_lifecycle("destroy")
        end
    else
        error(string.format("WoodMan's attack's phase %#x reads past its table (0x080C9908)", phase))
    end
end

-- 0x080C98A0: his action; off the field, he goes.
local function tick(me: Object)
    if me.action == APPEAR then
        appear(me)
    elseif me.action == ATTACK then
        attack(me)
    else
        error(string.format("WoodMan's action %#x reads past his table (0x080C98C4)", me.action))
    end
    if not field.valid(me.panel_x, me.panel_y) then
        me:set_lifecycle("destroy")
    end
end

-- 0x080C9850: his sprite (08-0C) with his shadow, shown, in animation 0, his chip's palette, facing his side's way, on
-- his panel; his sound; 20 ticks; then his first tick.
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
    me.timer = 20
    me:set_lifecycle("update")
    tick(me)
end

-- A puff 16 pixels above him (effect row 0x13).
local PUFF = new.effect { sprite = asset.sprite("puff"), anim = 0, palette = 0 }

woodman.kind = new.kind {
    id = "woodman/navi",
    pool = "actor",
    state = { controller = "object", palette = "u8", down = "u8", a = "u8", b = "u8", again = "bool" },
    -- 0x080C980C: by his state, then his sprite, while dimmed too.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
        elseif l == "update" then
            tick(me)
        else
            -- 0x080C9830: a puff 16 pixels above him; the controller moves on; he is freed.
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

-- 0x080C9A70: WoodMan for the controller's user, with the damage plus the bonus, in the chip's palette. (His position
-- is the spawner's registers: the row, the element and 0.)
local function spawn(controller: Object, damage: number, palette: number): Object?
    local o = battle.spawn(woodman.kind, Vec3.new(controller.panel_y, controller.element, 0))
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

-- What each WoodMan chip states for its own controller: the controller's id (the original's effect object #0x70 for
-- all three: compat maps each to it) and his palette (0x0800B950 by the chip's level: 0, the SP's 4, the DS's 6).
export type ChipSpec = { id: string, palette: number }

-- A WoodMan chip's controller: the short course (the user warps out, 11 ticks, WoodMan, 21 ticks after he left, the
-- user warps back in).
local function controller(chip: ChipSpec): NaviChipController
    return navi_chips.controller {
        id = chip.id,
        name = "WoodMan",
        spawn = function(c: Object, _user: Object, damage: number): Object?
            return spawn(c, damage, chip.palette)
        end,
        short = navi_chips.SHORT,
    }
end

return { controller = controller }
