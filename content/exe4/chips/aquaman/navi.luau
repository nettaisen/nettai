--!strict
-- AquaMan, the navi of EXE4's AquaMan chips (EXE4's actor object #0x1A, 0x080BAF88: EXE4's own, on EXE6's navi
-- template), and each chip's controller, which brings him (effect object #0x16, 0x080E3958: lib/navi_chips' long
-- course). He appears with the spout on his head (EXE4's actor #0x5C, @exelib/numbrman/face's object, its sprite
-- row 1) and, off a panel that isn't solid, goes at once; stands 20 ticks; then, 10 ticks into his pose, sets his
-- water gun (./water_gun) on the panel ahead and waits for it to go, then 21 ticks more; then he goes.

local navi_chips = require("../../lib/navi_chips")
local water_gun = require("./water_gun")

local aquaman = {}

type State = {
    -- The controller waiting for him (the flag it gave him, his +0x50).
    controller: Object?,
    -- His chip's palette.
    palette: number,
    -- The spout on his head (+0x60).
    spout: Object?,
    -- His water gun while it stands (+0x64: it empties it as it goes).
    gun: Object?,
}

local SPRITE = asset.sprite("aquaman")
local SOUND_APPEAR = asset.sound("appear")

-- The spout on his head (actor #0x5C's sprite row 1, 10-0C; its Param2 1: with the fight on, it follows him from its
-- first update).
local SPOUT = require("@exelib/numbrman/face").make {
    id = "aquaman/spout",
    sprite = asset.sprite("aquaman-spout"),
    follows_in_fight = true,
}

local APPEAR, STAND, ATTACK, LEAVE = 0x0, 0x4, 0x8, 0xC

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

-- Action 0 (0x080BB024): shown, animation 3, his sound; 5 ticks; then he stays, or goes off a panel that isn't solid.
local function appear(me: Object)
    if me.phase_init == 0 then
        me.visible = true
        me.anim = 3
        battle.play_sound(SOUND_APPEAR)
    end
    if countdown(me, 5, false) then
        me:set_action(if field.solid(me.panel_x, me.panel_y) then STAND else LEAVE)
    end
end

-- Action 4 (0x080BB062): 20 ticks standing (animation 0).
local function stand(me: Object)
    if me.phase_init == 0 then
        me.anim = 0
    end
    if countdown(me, 20, false) then
        me:set_action(ATTACK)
    end
end

-- Action 8 (0x080BB088), phase 0 (0x080BB0A4): his pose (animation 10), 10 ticks; then his water gun on the panel
-- ahead, of his element, with his damage word. Phase 4 (0x080BB0DE): while it stands, nothing; then 21 ticks.
local function attack(me: Object)
    local s = me.state :: State
    if me.phase == 0 then
        if me.phase_init == 0 then
            me.anim = 10
        end
        if countdown(me, 10, false) then
            water_gun.spawn(me, me.panel_x + me.facing, me.panel_y, me.element,
                bit32.bor(me.damage, bit32.lshift(me.stamina, 16)))
            me.phase, me.phase_init = 4, 0
        end
    elseif me.phase == 4 then
        if s.gun ~= nil then
            return
        end
        if countdown(me, 20, true) then
            me:set_action(LEAVE)
        end
    else
        error(string.format("AquaMan's attack's phase %#x reads past its table (0x080BB09C)", me.phase))
    end
end

-- Action 0xC (0x080BB104): animation 4; 11 ticks; then his spout goes, the controller moves on, and he goes.
local function leave(me: Object)
    if me.phase_init == 0 then
        me.anim = 4
    end
    if countdown(me, 10, true) then
        local s = me.state :: State
        local spout = s.spout
        if spout then
            spout:set_lifecycle("destroy")
        end
        if s.controller then
            navi_chip.navi_left(s.controller :: Object)
        end
        -- (The state byte alone: his action and phase stay.)
        me.lifecycle = "destroy"
    end
end

local ACTIONS: { [number]: (Object) -> () } = { [APPEAR] = appear, [STAND] = stand, [ATTACK] = attack, [LEAVE] = leave }

-- 0x080BAFAC: his sprite (08-0B) with his shadow, his chip's palette, facing his side's way, on his panel, on the
-- ground; the spout on his head. (The original stores 0xFF in his animation where it meant its copy, and loads that
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
    s.spout = SPOUT.spawn(me)
    me:set_lifecycle("update")
end

aquaman.kind = new.kind {
    id = "aquaman/navi",
    pool = "actor",
    state = { controller = "object", palette = "u8", spout = "object", gun = "object" },
    -- 0x080BAF88: by his state, then his sprite, while dimmed too.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
        elseif l == "update" then
            local f = ACTIONS[me.action]
            if not f then
                error(string.format("AquaMan's action %#x reads past his table (0x080BB014)", me.action))
            end
            f(me)
        else
            me:free()
            return
        end
        me:update_sprite_while_dimmed()
    end,
}

-- 0x080BB132: AquaMan for the controller's user, with the damage plus the bonus, in the chip's palette. (His position
-- is the spawner's registers: the row, the element and whatever the controller left in r3, which his init overwrites.
-- The controller gives him B's charge at the ask, full or not, as his Param2, which he never reads: 0x080E39FA.)
local function spawn(controller: Object, damage: number, palette: number): Object?
    local o = battle.spawn(aquaman.kind, Vec3.new(controller.panel_y, controller.element, 0))
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

-- What each AquaMan chip states for its own controller: the controller's id (the original's effect object #0x16 for
-- all three: compat maps each to it) and his palette (0x0800B950 by the chip's level: 0, the SP's 2, the DS's 3).
export type ChipSpec = { id: string, palette: number }

-- An AquaMan chip's controller: the long course (the user warps out, AquaMan, 31 ticks, the user warps back in).
local function controller(chip: ChipSpec): NaviChipController
    return navi_chips.controller {
        id = chip.id,
        name = "AquaMan",
        spawn = function(c: Object, _user: Object, damage: number): Object?
            return spawn(c, damage, chip.palette)
        end,
        steps = navi_chips.LONG,
    }
end

return { controller = controller }
