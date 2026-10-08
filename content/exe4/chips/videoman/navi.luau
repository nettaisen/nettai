--!strict
-- VideoMan, the navi of EXE4's VideoMan chips (EXE4's actor object #0x3E, 0x080C76EC: EXE4's own), and each chip's
-- controller, which brings him (effect object #0x5F, 0x080E8BA8: lib/navi_chips' short course). He appears on his
-- user's panel (his sound) and, 21 ticks later, swings (animation 5): as it ends, a tape (./tape) over the square two
-- panels ahead of him and a row up (but on the top row), and 45 ticks into his swing another over the square two ahead
-- in his row (but on the bottom row); 111 ticks after his swing began he goes in a puff. (For side 1 the squares are three panels
-- back from him: each square spreads to the right.)

local navi_chips = require("../../lib/navi_chips")
local tape = require("./tape")

local videoman = {}

type State = {
    -- The controller waiting for him (the flag it gave him, his +0x4C).
    controller: Object?,
    -- His chip's palette.
    palette: number,
}

local SPRITE = asset.sprite("videoman")
local SOUND_APPEAR = asset.sound("appear")
-- The puff he goes in (effect row 0x13).
local PUFF = new.effect { sprite = asset.sprite("puff"), anim = 0, palette = 0 }

local APPEAR, SWING = 0x0, 0x4

-- His squares' column: two ahead of him for side 0, three back for side 1 (0x080C77DE).
local function square_x(me: Object): number
    return me.panel_x + if me.alliance == 0 then 2 else -3
end

-- Action 0 (0x080C77AC): 21 ticks; then his swing (animation 5), 111 ticks.
local function appear(me: Object)
    local left = me.timer - 1
    me.timer = int.u16(left)
    if left >= 0 then
        return
    end
    me.anim = 5
    me.timer = 110
    me:set_action(SWING)
end

-- Action 4 (0x080C77C8): as his swing ends, animation 6 and the upper tape; else with 65 ticks left the lower one;
-- with 20 left animation 7; at its end he goes.
local function swing(me: Object)
    if me.anim == 5 and bit32.btest(me.sprite.frame_flags, 0x80) then
        me.anim = 6
        if me.panel_y ~= 1 then
            tape.spawn(me, square_x(me), me.panel_y - 1)
        end
    elseif me.timer == 65 then
        if me.panel_y ~= 3 then
            tape.spawn(me, square_x(me), me.panel_y)
        end
    end
    if me.timer == 20 then
        me.anim = 7
    end
    local left = me.timer - 1
    me.timer = int.u16(left)
    if left < 0 then
        me:set_lifecycle("destroy")
    end
end

-- 0x080C7790.
local function tick(me: Object)
    if me.action == APPEAR then
        appear(me)
    elseif me.action == SWING then
        swing(me)
    else
        error(string.format("VideoMan's action %#x reads past his table (0x080C77A4)", me.action))
    end
end

-- 0x080C7730: his sprite (08-17) with his shadow, shown, in animation 0, his chip's palette, facing his side's way, on
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

videoman.kind = new.kind {
    id = "videoman/navi",
    pool = "actor",
    state = { controller = "object", palette = "u8" },
    -- 0x080C76EC: by his state, then his sprite, while dimmed too.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
        elseif l == "update" then
            tick(me)
        else
            -- 0x080C7710: a puff 16 pixels above him (effect row 0x13); the controller moves on; he is freed.
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

-- 0x080C7844: VideoMan for the controller's user, with the damage plus the bonus, in the chip's palette. (His position
-- is the spawner's registers: the row, the element and 0.)
local function spawn(controller: Object, damage: number, palette: number): Object?
    local o = battle.spawn(videoman.kind, Vec3.new(controller.panel_y, controller.element, 0))
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

-- What each VideoMan chip states for its own controller: the controller's id (the original's effect object #0x5F for
-- all three: compat maps each to it) and his palette (0x080C775C by the chip's level: 0, the SP's 5, the DS's 6).
export type ChipSpec = { id: string, palette: number }

-- A VideoMan chip's controller: the short course (the user warps out, 11 ticks, VideoMan, 21 ticks after he left, the
-- user warps back in).
local function controller(chip: ChipSpec): NaviChipController
    return navi_chips.controller {
        id = chip.id,
        name = "VideoMan",
        spawn = function(c: Object, _user: Object, damage: number): Object?
            return spawn(c, damage, chip.palette)
        end,
        short = navi_chips.SHORT,
    }
end

return { controller = controller }
