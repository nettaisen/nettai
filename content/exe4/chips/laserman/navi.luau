--!strict
-- LaserMan, the navi of EXE4's LaserMan chips (EXE4's actor object #0x4E, 0x080CABE4: EXE4's own), and each chip's
-- controller, which brings him (effect object #0x75, 0x080EA3F8: lib/navi_chips' short course). He appears on his
-- user's panel and, 21 ticks later (gone if it isn't solid), charges (animation 2, 31 ticks), then fires his laser
-- (./laser) from the panel ahead along his row (animation 3, its sound) and stands 81 ticks; then he goes as his last
-- animation (4) ends. With LasrMnSP and LasrMnDS, the direction his side's navi holds while dimmed as he charges (up,
-- down, right or left: the first held in that order) aims the laser's course: the NaviCust programs it strips.

local laser = require("./laser")
local navi_chips = require("../../lib/navi_chips")

local laserman = {}

type State = {
    -- The controller waiting for him (the flag it gave him, his +0x4C).
    controller: Object?,
    -- His chip's palette, and its chip aims (its level, Param1, not 0).
    palette: number,
    aims: boolean,
    -- His laser's course (Param2: 0 ahead, then up, down, right, left).
    course: number,
}

local SPRITE = asset.sprite("laserman")
local SOUND_APPEAR = asset.sound("appear")
local SOUND_FIRE = asset.sound("laser")
-- The puff he goes in (effect row 0x13).
local PUFF = new.effect { sprite = asset.sprite("puff"), anim = 0, palette = 0 }
local SOLID = 0x10
-- The courses the keys held aim, in the order they are tried (0x080CAD1C).
local AIMS: { { key: Key, course: number } } = {
    { key = "up", course = 1 },
    { key = "down", course = 2 },
    { key = "right", course = 3 },
    { key = "left", course = 4 },
}

local APPEAR, ATTACK = 0x0, 0x4

-- Action 0 (0x080CACB4): 21 ticks; then, on a solid panel, his attack; else he goes.
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

-- Action 4 (0x080CACDE): phase 0 (0x080CACFC) charging (animation 2), aiming as his side's navi holds a direction, 31
-- ticks; then the laser, animation 3, its sound; phase 4 (0x080CAD72) 81 ticks, then animation 4; phase 8
-- (0x080CAD86) as it ends, he goes.
local function attack(me: Object)
    local s = me.state :: State
    if me.phase == 0 then
        if me.phase_init == 0 then
            me.phase_init = 1
            me.anim = 2
            me.timer = 30
        end
        if s.aims then
            local navi = battle.player(me.alliance)
            if navi then
                for _, aim in AIMS do
                    if navi:dimmed_held(aim.key) then
                        s.course = aim.course
                        break
                    end
                end
            end
        end
        local left = me.timer - 1
        me.timer = int.u16(left)
        if left >= 0 then
            return
        end
        me.anim = 3
        laser.spawn(me, me.panel_x + me.facing, me.panel_y, s.course)
        me.timer = 80
        battle.play_sound(SOUND_FIRE)
        me.phase, me.phase_init = 4, 0
    elseif me.phase == 4 then
        local left = me.timer - 1
        me.timer = int.u16(left)
        if left < 0 then
            me.anim = 4
            me.phase, me.phase_init = 8, 0
        end
    elseif me.phase == 8 then
        if bit32.btest(me.sprite.frame_flags, 0x80) then
            me:set_lifecycle("destroy")
        end
    else
        error(string.format("LaserMan's attack's phase %#x reads past its table (0x080CACF0)", me.phase))
    end
end

-- 0x080CAC8A: his action; off the field, he goes.
local function tick(me: Object)
    if me.action == APPEAR then
        appear(me)
    elseif me.action == ATTACK then
        attack(me)
    else
        error(string.format("LaserMan's action %#x reads past his table (0x080CACAC)", me.action))
    end
    if not field.valid(me.panel_x, me.panel_y) then
        me:set_lifecycle("destroy")
    end
end

-- 0x080CAC28: his sprite (08-16) with his shadow, shown, in animation 0, his chip's palette, facing his side's way, on
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

laserman.kind = new.kind {
    id = "laserman/navi",
    pool = "actor",
    state = { controller = "object", palette = "u8", aims = "bool", course = "u8" },
    -- 0x080CABE4: by his state, then his sprite, while dimmed too.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
        elseif l == "update" then
            tick(me)
        else
            -- 0x080CAC08: a puff 16 pixels above him; the controller moves on; he is freed.
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

-- 0x080CAD98: LaserMan for the controller's user, with the damage plus the bonus, in the chip's palette. (His position
-- is the spawner's registers: the row, the element and 0.)
local function spawn(controller: Object, damage: number, chip: ChipSpec): Object?
    local o = battle.spawn(laserman.kind, Vec3.new(controller.panel_y, controller.element, 0))
    if not o then
        return nil
    end
    o.panel_x, o.panel_y = controller.panel_x, controller.panel_y
    o.element = controller.element
    o.alliance = controller.alliance
    o:set_damage_word(damage)
    local s = o.state :: State
    s.controller = controller
    s.palette = chip.palette
    s.aims = chip.aims
    s.course = 0
    return o
end

-- What each LaserMan chip states for its own controller: the controller's id (the original's effect object #0x75 for
-- all three: compat maps each to it), his palette (0x080CAC54 by the chip's level: 0, the SP's 10, the DS's 11) and
-- whether he aims (the SP's and the DS's: a level not 0).
export type ChipSpec = { id: string, palette: number, aims: boolean }

-- A LaserMan chip's controller: the short course (the user warps out, 11 ticks, LaserMan, 21 ticks after he left, the
-- user warps back in).
local function controller(chip: ChipSpec): NaviChipController
    return navi_chips.controller {
        id = chip.id,
        name = "LaserMan",
        spawn = function(c: Object, _user: Object, damage: number): Object?
            return spawn(c, damage, chip)
        end,
        short = navi_chips.SHORT,
    }
end

return { controller = controller }
