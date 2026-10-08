--!strict
-- WindMan, the navi of EXE4's WindMan chips (EXE4's actor object #0x34, 0x080C3DA8: EXE4's own, on EXE6's navi
-- template), and each chip's controller, which brings him (effect object #0x46, 0x080E6E68: lib/navi_chips' long
-- course with a wait of 61 ticks). He appears with the plume on his head (EXE4's actor #0x5C, @exelib/numbrman/face's
-- object, its sprite row 2) and stands 20 ticks; then, three times, 11 ticks and a blow (animation 16, 16 ticks)
-- that sends a tornado (./tornado) from the panel ahead 5 ticks in; 11 ticks on, he goes.

local navi_chips = require("../../lib/navi_chips")
local tornado = require("./tornado")

local windman = {}

type State = {
    -- The controller waiting for him (the flag it gave him, his +0x50).
    controller: Object?,
    -- His chip's palette.
    palette: number,
    -- The plume on his head (+0x60).
    plume: Object?,
}

local SPRITE = asset.sprite("windman")
local SOUND_APPEAR = asset.sound("appear")

-- The plume on his head (actor #0x5C's sprite row 2, 10-28; its Param2 1: with the fight on, it follows him from its
-- first update). (Row 2's follow sets its animation 1 while the object at its owner's +0x64 has Param1 0: WindMan
-- never sets one, so it stays at 0, as rows 0 and 1 keep theirs.)
local PLUME = require("@exelib/numbrman/face").make {
    id = "windman/plume",
    sprite = asset.sprite("windman-plume"),
    follows_in_fight = true,
}

local APPEAR, STAND, ATTACK, LEAVE = 0x0, 0x4, 0x8, 0xC
-- His blows.
local BLOWS = 3

-- Enter the phase with a timer, or count down; true once it has run out: past 0 (`bge`), or at 0 (`bgt`).
local function countdown(me: Object, ticks: number, past: boolean): boolean
    if me.phase_init == 0 then
        me.phase_init = 4
        me.timer = ticks
    end
    local left = me.timer - 1
    me.timer = int.u16(left)
    return if past then left < 0 else left <= 0
end

-- Action 0 (0x080C3E4C): shown, animation 3, his sound; 5 ticks; then he stands (whatever his panel).
local function appear(me: Object)
    if me.phase_init == 0 then
        me.visible = true
        me.anim = 3
        battle.play_sound(SOUND_APPEAR)
    end
    if countdown(me, 5, false) then
        me:set_action(STAND)
    end
end

-- Action 4 (0x080C3E80): 20 ticks standing (animation 0).
local function stand(me: Object)
    if me.phase_init == 0 then
        me.anim = 0
    end
    if countdown(me, 20, false) then
        me:set_action(ATTACK)
    end
end

-- Action 8 (0x080C3EA6): phase 0 (0x080C3EC8) his three blows; phase 4 (0x080C3ED2) 11 ticks; phase 8 (0x080C3EEE)
-- a blow (animation 16 from its start), with 10 of its 16 ticks left a tornado from the panel ahead; then another, or
-- phase 0xC (0x080C3F3E): 11 ticks, then he leaves.
local function attack(me: Object)
    if me.phase == 0 then
        me.timer2 = BLOWS
        me.phase, me.phase_init = 4, 0
    elseif me.phase == 4 then
        if countdown(me, 10, true) then
            me.phase, me.phase_init = 8, 0
        end
    elseif me.phase == 8 then
        if me.phase_init == 0 then
            me.anim, me.anim_loaded = 16, 0xFF
            me.timer = 15
            me.phase_init = 4
        end
        if me.timer == 10 then
            tornado.spawn(me, me.panel_x + me.facing, me.panel_y, me.element)
        end
        if countdown(me, 15, true) then
            me.timer2 = int.u16(me.timer2 - 1)
            me.phase, me.phase_init = if int.i16(me.timer2) > 0 then 4 else 0xC, 0
        end
    elseif me.phase == 0xC then
        if countdown(me, 10, true) then
            me:set_action(LEAVE)
        end
    else
        error(string.format("WindMan's attack's phase %#x reads past its table (0x080C3EB8)", me.phase))
    end
end

-- Action 0xC (0x080C3F60): animation 4; 4 ticks; then his plume goes, the controller moves on, and he goes.
local function leave(me: Object)
    if me.phase_init == 0 then
        me.anim = 4
    end
    if countdown(me, 3, true) then
        local s = me.state :: State
        local plume = s.plume
        if plume then
            plume:set_lifecycle("destroy")
        end
        if s.controller then
            navi_chip.navi_left(s.controller :: Object)
        end
        -- (The state byte alone: his action and phase stay.)
        me.lifecycle = "destroy"
    end
end

local ACTIONS: { [number]: (Object) -> () } = { [APPEAR] = appear, [STAND] = stand, [ATTACK] = attack, [LEAVE] = leave }

-- 0x080C3DCC: his sprite (08-03) with his shadow, his chip's palette, facing his side's way, on his panel, on the
-- ground; the plume on his head. (The original stores 0xFF in his animation where it meant its copy, and loads that
-- animation: nothing until his first action sets one.)
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
    s.plume = PLUME.spawn(me)
    me:set_lifecycle("update")
end

windman.kind = new.kind {
    id = "windman/navi",
    pool = "actor",
    state = { controller = "object", palette = "u8", plume = "object" },
    -- 0x080C3DA8: by his state, then his sprite, while dimmed too.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
        elseif l == "update" then
            local f = ACTIONS[me.action]
            if not f then
                error(string.format("WindMan's action %#x reads past his table (0x080C3E3C)", me.action))
            end
            f(me)
        else
            me:free()
            return
        end
        me:update_sprite_while_dimmed()
    end,
}

-- 0x080C3F8E: WindMan for the controller's user, with the damage plus the bonus, in the chip's palette. (His position
-- is the spawner's registers: the row, the element and whatever the controller left in r3, which his init overwrites.)
local function spawn(controller: Object, damage: number, palette: number): Object?
    local o = battle.spawn(windman.kind, Vec3.new(controller.panel_y, controller.element, 0))
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

-- What each WindMan chip states for its own controller: the controller's id (the original's effect object #0x46 for
-- all three: compat maps each to it) and his palette (0x0800B950 by the chip's level: 0, the SP's 4, the DS's 6).
export type ChipSpec = { id: string, palette: number }

-- His controller's course (0x080E6EC4): the user warps out (31 ticks), WindMan, 61 ticks, the user warps back in.
local STEPS = { { wait = 0x1E, warp = "out" }, "navi", { wait = 0x3C }, { wait = 0x1E, warp = "in" } } :: { NaviChipStep }

-- A WindMan chip's controller.
local function controller(chip: ChipSpec): NaviChipController
    return navi_chips.controller {
        id = chip.id,
        name = "WindMan",
        spawn = function(c: Object, _user: Object, damage: number): Object?
            return spawn(c, damage, chip.palette)
        end,
        steps = STEPS,
    }
end

return { controller = controller }
