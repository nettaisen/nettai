--!strict
-- FireMan, the navi of EXE4's FireMan chips (EXE4's actor object #0x12, 0x080B92A2: EXE4's own, on EXE6's navi
-- template), and each chip's controller, which brings him (effect object #0x0F, 0x080E30BC: lib/navi_chips' long
-- course). He appears with the flame on his head (a form overlay, the engine's: actor #0x57, sprite 10-08, stepping
-- while dimmed) and, off a panel that isn't solid, goes at once; stands 20 ticks; then breathes fire for 90 ticks:
-- the Elem chips' flame (attack #9: @exelib/elemrage/flame, his own kind) from the panel ahead, five panels along his
-- row, each for 80 ticks, a roar each 16 ticks. His command, Down then Right held alone while he appears and stands,
-- makes his flames leave lava.

local collision = require("../../rules/collision")
local flame_common = require("@exelib/elemrage/flame")
local navi_chips = require("../../lib/navi_chips")
local sparks = require("../../lib/sparks")

local fireman = {}

type State = {
    -- The controller waiting for him (the flag it gave him, his +0x50).
    controller: Object?,
    -- His chip's palette.
    palette: number,
    -- The flame on his head (+0x60).
    flame: Object?,
    -- His command: its steps entered (+0x64), and done (Param4).
    command_steps: number,
    commanded: boolean,
}

local SPRITE = asset.sprite("fireman")
local HEAD_FLAME = asset.sprite("fireman-flame")
local SOUND_APPEAR = asset.sound("appear")
local SOUND_ROAR = asset.sound("roar")

-- His flames (attack object #9, the Elem chips' flame, 0x080CE2D8): its sprite (0c-0e), lava left with his command.
local FLAME = flame_common.make_kind {
    id = "fireman/flame",
    sprite = asset.sprite("rage-flame"),
    leaves = { "lava" },
    spreads = false,
    attack = collision.attack,
    attack_480 = collision.attack_480,
    hits = collision.hits_navis,
    spark = sparks.of_element,
}
-- His flames (0x080B9400: Param1 5, Param2 0x50, Param3 0xF0, Param4 1, its high nibble 8 with his command): five
-- along his row, each 80 ticks, 16 pixels back from its panel's center, running while dimmed; fire, 34 pixels up.
local FLAMES, FLAME_TICKS, FLAME_OFFSET, FLAME_FLAGS = 5, 0x50, -16, 1
local FIRE, FLAME_Z = 1, 0x22_0000

-- His command (0x080B9298's one sequence, 0x080B94A8): each key held alone, while dimmed, in turn.
local COMMAND: { Key } = { "down", "right" }
local KEYS: { Key } = { "a", "b", "select", "start", "right", "left", "up", "down", "r", "l" }

local APPEAR, STAND, ATTACK, LEAVE = 0x0, 0x4, 0x8, 0xC

-- Whether `navi` holds `key` alone (its dimmed record's word, 0x3FF of it, is the key's).
local function held_alone(navi: Object, key: Key): boolean
    for _, k in KEYS do
        if navi:dimmed_held(k) ~= (k == key) then
            return false
        end
    end
    return true
end

-- 0x080B94A8 (read unless event flag 0x1187, which no netbattle sets, is on): his side's navi holding the command's
-- next key alone moves it a step; done, his flames leave lava.
local function command(me: Object)
    local s = me.state :: State
    local navi = battle.player(me.alliance)
    if not navi then
        return
    end
    local key = COMMAND[s.command_steps + 1]
    if key and held_alone(navi, key) then
        s.command_steps += 1
        key = COMMAND[s.command_steps + 1]
    end
    if not key then
        s.commanded = true
    end
end

-- Enter the phase with a timer (counting down from its first tick), or count down; true once it has run out.
local function countdown(me: Object, ticks: number): boolean
    if me.phase_init == 0 then
        me.phase_init = 4
        me.timer = ticks
    end
    local left = me.timer - 1
    me.timer = int.u16(left)
    return left <= 0
end

-- Action 0 (0x080B9344): his command read; shown, animation 3, his sound; 5 ticks; then he stays, or goes off a panel
-- that isn't solid.
local function appear(me: Object)
    command(me)
    if me.phase_init == 0 then
        me.visible = true
        me.anim = 3
        battle.play_sound(SOUND_APPEAR)
    end
    if countdown(me, 5) then
        me:set_action(if field.solid(me.panel_x, me.panel_y) then STAND else LEAVE)
    end
end

-- Action 4 (0x080B93A4): his command read; 20 ticks standing (animation 0).
local function stand(me: Object)
    command(me)
    if me.phase_init == 0 then
        me.anim = 0
    end
    if countdown(me, 20) then
        me:set_action(ATTACK)
    end
end

-- Action 8 (0x080B93EC): animation 10, his flames from the panel ahead; a roar each 16 ticks; 90 ticks.
local function attack(me: Object)
    local s = me.state :: State
    if me.phase_init == 0 then
        me.anim = 10
        me.timer = 90
        me.timer2 = 0
        FLAME.spawn(me, me.panel_x + me.facing, me.panel_y, FIRE, FLAME_Z, {
            count = FLAMES,
            spread = false,
            ticks = FLAME_TICKS,
            offset = FLAME_OFFSET,
            flags = FLAME_FLAGS,
            leaves = if s.commanded then "lava" else nil,
            palette = 0,
            hit = false,
            sent = false,
        }, bit32.bor(me.damage, bit32.lshift(me.stamina, 16)))
        me.phase_init = 4
    end
    me.timer2 = bit32.band(me.timer2 + 1, 15)
    if me.timer2 == 1 then
        battle.play_sound(SOUND_ROAR)
    end
    local left = me.timer - 1
    me.timer = int.u16(left)
    if left <= 0 then
        me:set_action(LEAVE)
    end
end

-- Action 0xC (0x080B9454): animation 4; 5 ticks; then his head's flame goes, the controller moves on, and he goes.
local function leave(me: Object)
    if me.phase_init == 0 then
        me.anim = 4
    end
    if countdown(me, 5) then
        local s = me.state :: State
        local flame = s.flame
        if flame then
            flame:set_lifecycle("destroy")
        end
        if s.controller then
            navi_chip.navi_left(s.controller :: Object)
        end
        -- (The state byte alone: his action and phase stay.)
        me.lifecycle = "destroy"
    end
end

local ACTIONS: { [number]: (Object) -> () } = { [APPEAR] = appear, [STAND] = stand, [ATTACK] = attack, [LEAVE] = leave }

-- 0x080B92C4: his sprite (08-05) with his shadow, his chip's palette, facing his side's way, on his panel, on the
-- ground; the flame on his head; his command not begun. (The original stores 0xFF in his animation where it meant its
-- copy, and loads that animation: nothing until his first action sets one.)
local function init(me: Object)
    local s = me.state :: State
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "ground"
    me.anim = 0xFF
    sprite:set_animation(0xFF)
    sprite.palette = s.palette
    sprite:set_flip(me.alliance)
    me:set_coordinates_from_panel()
    local p = me.pos
    me.pos = Vec3.new(p.x, p.y, 0)
    s.flame = battle.form_overlay(me, { sprite = HEAD_FLAME, stepping = "while_dimmed", owner_palette = true })
    s.command_steps = 0
    s.commanded = false
    me:set_lifecycle("update")
end

fireman.kind = new.kind {
    id = "fireman/navi",
    pool = "actor",
    state = { controller = "object", palette = "u8", flame = "object", command_steps = "u8", commanded = "bool" },
    -- 0x080B92A2: by his state, then his sprite, while dimmed too.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
        elseif l == "update" then
            local f = ACTIONS[me.action]
            if not f then
                error(string.format("FireMan's action %#x reads past his table (0x080B9334)", me.action))
            end
            f(me)
        else
            me:free()
            return
        end
        me:update_sprite_while_dimmed()
    end,
}

-- 0x080B9482: FireMan for the controller's user, with the damage plus the bonus, in the chip's palette. (His position
-- is the spawner's registers: the row, the element and whatever the controller left in r3, which his init overwrites.)
local function spawn(controller: Object, damage: number, palette: number): Object?
    local o = battle.spawn(fireman.kind, Vec3.new(controller.panel_y, controller.element, 0))
    if not o then
        return nil
    end
    o.panel_x, o.panel_y = controller.panel_x, controller.panel_y
    o.element = controller.element
    o.related1 = controller
    o.alliance = controller.alliance
    o:set_damage_word(damage)
    local s = o.state :: State
    s.controller = controller
    s.palette = palette
    return o
end

-- What each FireMan chip states for its own controller: the controller's id (the original's effect object #0x0F for
-- all three: compat maps each to it) and his palette (0x0800B950 by the chip's level: 0, the SP's 2, the DS's 3).
export type ChipSpec = { id: string, palette: number }

-- A FireMan chip's controller: the long course (the user warps out, FireMan, 31 ticks, the user warps back in).
local function controller(chip: ChipSpec): NaviChipController
    return navi_chips.controller {
        id = chip.id,
        name = "FireMan",
        spawn = function(c: Object, _user: Object, damage: number): Object?
            return spawn(c, damage, chip.palette)
        end,
        steps = navi_chips.LONG,
    }
end

return { controller = controller }
