--!strict
-- GutsMan, the navi of EXE4's GutsMan chips (EXE4's actor object #0x14, 0x080B999C: EXE4's own, on EXE6's navi
-- template), and each chip's controller, which brings him (effect object #0x10, 0x080E31E8: lib/navi_chips' long
-- course). He appears and, off a panel that isn't solid, goes at once; stands 20 ticks; then swings his hammer (90
-- ticks): 20 ticks in it strikes the panel ahead (a heavy hit) and sets off his quake there (./quake), which cracks
-- the panels ahead and drops rocks on the other side.

local collision = require("../../rules/collision")
local navi_chips = require("../../lib/navi_chips")
local quake = require("./quake")
local regions = require("@exelib/regions")
local sparks = require("../../lib/sparks")

local gutsman = {}

type State = {
    -- The controller waiting for him (the flag it gave him, his +0x50).
    controller: Object?,
    -- His chip's palette.
    palette: number,
    -- His quake cracks all the panels ahead, not six (his chip's level, Param1, 3 or more: 0x080B9AA8).
    cracks_all: boolean,
}

local SPRITE = asset.sprite("gutsman")
local SOUND_APPEAR = asset.sound("appear")

local APPEAR, STAND, HAMMER, LEAVE = 0x0, 0x4, 0x8, 0xC

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

-- Action 0 (0x080B9A28): shown, animation 3, his sound; 5 ticks; then he stays, or goes off a panel that isn't solid.
local function appear(me: Object)
    if me.phase_init == 0 then
        me.visible = true
        me.anim = 3
        battle.play_sound(SOUND_APPEAR)
    end
    if countdown(me, 5) then
        me:set_action(if field.solid(me.panel_x, me.panel_y) then STAND else LEAVE)
    end
end

-- Action 4 (0x080B9A66): 20 ticks standing (animation 0).
local function stand(me: Object)
    if me.phase_init == 0 then
        me.anim = 0
    end
    if countdown(me, 20) then
        me:set_action(HAMMER)
    end
end

-- His hammer's strike (0x080B9AB0): his quake on the panel ahead, and a hit there (`sub_80C53A6`: single, the heavy
-- spark, an attack of his element at navis 16 pixels up, hit modifier 1), resolving while dimmed.
local function strike(me: Object)
    local s = me.state :: State
    local x = me.panel_x + me.facing
    quake.spawn(me, x, me.panel_y, s.cracks_all)
    local box = battle.hitbox(me, {
        panel_x = x,
        panel_y = me.panel_y,
        element = me.element,
        z = 0x10_0000,
        region = regions.single,
        hit_effect = sparks.heavy,
        target = collision.hits_navis,
        self_type = collision.attack,
        damage = me.damage,
        stamina = me.stamina,
        hit_mod = 1,
    })
    if box then
        box.run_while_dimmed = true
    end
end

-- Action 8 (0x080B9A8C): his hammer (animation 10), 90 ticks; with 70 left, its strike.
local function hammer(me: Object)
    if me.phase_init == 0 then
        me.anim = 10
        me.timer = 90
        me.phase_init = 4
    end
    if me.timer == 70 then
        strike(me)
    end
    local left = me.timer - 1
    me.timer = int.u16(left)
    if left <= 0 then
        me:set_action(LEAVE)
    end
end

-- Action 0xC (0x080B9AF4): animation 4; 5 ticks; then the controller moves on, and he goes.
local function leave(me: Object)
    if me.phase_init == 0 then
        me.anim = 4
    end
    if countdown(me, 5) then
        local s = me.state :: State
        if s.controller then
            navi_chip.navi_left(s.controller :: Object)
        end
        -- (The state byte alone: his action and phase stay.)
        me.lifecycle = "destroy"
    end
end

local ACTIONS: { [number]: (Object) -> () } = { [APPEAR] = appear, [STAND] = stand, [HAMMER] = hammer, [LEAVE] = leave }

-- 0x080B99C0: his sprite (08-02) with his shadow, his chip's palette, facing his side's way, on his panel, on the
-- ground. (The original stores 0xFF in his animation where it meant its copy, and loads that animation: nothing until
-- his first action sets one.)
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
    me:set_lifecycle("update")
end

gutsman.kind = new.kind {
    id = "gutsman/navi",
    pool = "actor",
    state = { controller = "object", palette = "u8", cracks_all = "bool" },
    -- 0x080B999C: by his state, then his sprite, while dimmed too.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
        elseif l == "update" then
            local f = ACTIONS[me.action]
            if not f then
                error(string.format("GutsMan's action %#x reads past his table (0x080B9A18)", me.action))
            end
            f(me)
        else
            me:free()
            return
        end
        me:update_sprite_while_dimmed()
    end,
}

-- 0x080B9B1C: GutsMan for the controller's user, with the damage plus the bonus, in the chip's palette. (His position
-- is the spawner's registers: the row, the element and whatever the controller left in r3, which his init overwrites.
-- The controller gives him B's charge at the ask, full or not, as his Param2, which he never reads: 0x080E328A.)
local function spawn(controller: Object, damage: number, chip: ChipSpec): Object?
    local o = battle.spawn(gutsman.kind, Vec3.new(controller.panel_y, controller.element, 0))
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
    s.palette = chip.palette
    s.cracks_all = chip.cracks == "all"
    return o
end

-- What each GutsMan chip states for its own controller: the controller's id (the original's effect object #0x10 for
-- all three: compat maps each to it), his palette (0x0800B950 by the chip's level: 0, the SP's 2, the DS's 3) and how
-- many panels his quake cracks (by the level: six at random, or the SP's and the DS's all).
export type ChipSpec = { id: string, palette: number, cracks: "six" | "all" }

-- A GutsMan chip's controller: the long course (the user warps out, GutsMan, 31 ticks, the user warps back in).
local function controller(chip: ChipSpec): NaviChipController
    return navi_chips.controller {
        id = chip.id,
        name = "GutsMan",
        spawn = function(c: Object, _user: Object, damage: number): Object?
            return spawn(c, damage, chip)
        end,
        steps = navi_chips.LONG,
    }
end

return { controller = controller }
