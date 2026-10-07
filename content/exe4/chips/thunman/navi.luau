--!strict
-- ThunMan, the navi of EXE4's ThunMan chips (EXE4's actor object #0x37, 0x080C5920: EXE4's own, on the navi chips'
-- template SearchMan's is built on), whom the shared navi chip controller (objects/navi_chip_controller, its entry 4)
-- brings. He fades in on his user's panel (and goes if it isn't solid), raises his arms (animation 15) for 31 ticks,
-- then calls a thunderbolt (./thunder) on each row of the column three ahead of him, 21 ticks apart, in his animation
-- 10; lowers them for 31 ticks, and goes.

local navi_chips = require("../../lib/navi_chips")
local thunder = require("./thunder")

local thunman = {}

export type State = {
    -- The controller waiting for him (the flag it gave him, +0x64).
    controller: Object?,
    -- The chip's level (its Param1: 0, 3 for SP, 4 for DS): his palette.
    level: number,
    -- The gray of his color shader, fading in (+0x19).
    gray: number,
    -- The row of his next thunderbolt (Param3).
    row: number,
}

local SPRITE = asset.sprite("thunderman")
local SOUND_APPEAR = asset.sound("appear")
-- His palettes a variant (0x0800B9B1's entry 6).
local PALETTES = 2
local FADE_GRAY = 0x1F
local SOLID = 0x10

-- Action 0, phase 0 (0x080C59D8): he fades in from gray 31, 2 a tick, flickering.
local function fade_in(me: Object)
    local s = me.state :: State
    if me.phase_init == 0 then
        s.gray = FADE_GRAY
        battle.play_sound(SOUND_APPEAR)
        me.timer = 0
        me.phase_init = 4
        return
    end
    me.visible = true
    local t = int.u16(me.timer + 1)
    me.timer = t
    if bit32.band(t, 1) ~= 0 then
        me.visible = false
    end
    local gray = s.gray - 2
    s.gray = int.u8(gray)
    if gray > 0 then
        return
    end
    me.visible = true
    s.gray = 0
    me.phase, me.phase_init = 4, 0
end

-- Action 0, phase 4 (0x080C5A28): 20 ticks; then the thunder, or he goes off a panel that isn't solid.
local function ready(me: Object)
    if me.phase_init == 0 then
        me.timer = 0x14
        me.phase_init = 4
        return
    end
    local left = me.timer - 1
    me.timer = int.u16(left)
    if left > 0 then
        return
    end
    if field.check(me.panel_x, me.panel_y, SOLID, 0) then
        me:set_action(4)
    else
        me:set_lifecycle("destroy")
    end
end

-- Action 4, phase 0 (0x080C5A80): his arms up (animation 15); 30 ticks.
local function raise(me: Object)
    if me.phase_init == 0 then
        me.anim = 15
        me.timer = 0x1E
        me.phase_init = 4
        return
    end
    local left = me.timer - 1
    me.timer = int.u16(left)
    if left <= 0 then
        me.phase, me.phase_init = 4, 0
    end
end

-- 0x080C5B3A: a thunderbolt on his row's panel of the column three ahead (by his side), breaking it.
local function call(me: Object, s: State)
    local x = me.panel_x - 3
    if me.alliance == 0 then
        x += 6
    end
    thunder.spawn(me, x, s.row, bit32.bor(me.damage, bit32.lshift(me.stamina, 16)), true)
end

-- Action 4, phase 4 (0x080C5AA4): animation 10; a thunderbolt on rows 1, 2 and 3, 20 ticks apart.
local function strike(me: Object)
    local s = me.state :: State
    if me.phase_init == 0 then
        me.anim = 10
        s.row = 1
        call(me, s)
        me.timer = 0x14
        me.phase_init = 4
        return
    end
    local left = me.timer - 1
    me.timer = int.u16(left)
    if left > 0 then
        return
    end
    local row = s.row + 1
    if row ~= 4 then
        s.row = row
        call(me, s)
        me.timer = 0x14
        return
    end
    me.phase, me.phase_init = 8, 0
end

-- Action 4, phase 8 (0x080C5AD8): his arms down; 30 ticks.
local function lower(me: Object)
    if me.phase_init == 0 then
        me.anim = 0
        me.timer = 0x1E
        me.phase_init = 4
        return
    end
    local left = me.timer - 1
    me.timer = int.u16(left)
    if left <= 0 then
        me:set_action(8)
    end
end

-- Action 8 (0x080C5B00): he goes (animation 4 from its first frame); 5 ticks, then out of sight.
local function leave(me: Object)
    if me.phase == 0 then
        me.anim, me.anim_loaded = 4, 0
        me.timer = 5
        me.phase = 4
        return
    end
    local left = me.timer - 1
    me.timer = int.u16(left)
    if left <= 0 then
        me.visible = false
        me:set_lifecycle("destroy")
    end
end

local ACTIONS: { [number]: { [number]: (Object) -> () } } = {
    [0] = { [0] = fade_in, [4] = ready },
    [4] = { [0] = raise, [4] = strike, [8] = lower },
}

-- 0x080C5994: the action; the sprite, while dimmed too; his color shader from the gray (0x080C5B52).
local function tick(me: Object)
    if me.action == 8 then
        leave(me)
    else
        local phases = ACTIONS[me.action]
        if not phases then
            error(string.format("ThunMan action %#x reads past his table (0x080C59AC)", me.action))
        end
        local f = phases[me.phase]
        if not f then
            error(string.format("ThunMan action %#x phase %#x reads past its table", me.action, me.phase))
        end
        f(me)
    end
    me:update_sprite_while_dimmed()
    me.sprite.color_shader = (me.state :: State).gray * 0x421
end

-- 0x080C5940: on his panel with his shadow, facing his side's way, his palette by the level, faded out; his height the
-- spawner's.
local function init(me: Object)
    local s = me.state :: State
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "ground"
    me.anim, me.anim_loaded = 0, 0
    sprite:set_animation(0)
    me:set_coordinates_from_panel()
    sprite:set_flip(me.alliance)
    sprite.palette = navi_chips.palette(PALETTES, s.level)
    sprite.color_shader = 0x7FFF
    me.visible = true
    me:set_lifecycle("update")
    tick(me)
end

thunman.kind = new.kind {
    id = "thunman/navi",
    pool = "actor",
    state = { controller = "object", level = "u8", gray = "u8", row = "u8" },
    -- 0x080C5920.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
        elseif l == "update" then
            tick(me)
        else
            -- 0x080C5B2C: the controller moves on, and he is freed.
            local controller = (me.state :: State).controller
            if controller then
                navi_chip.navi_left(controller)
            end
            me:free()
        end
    end,
}

-- 0x080C5B64: ThunMan for the controller's user, with the damage plus the bonus, of the chip's level. (His position is
-- the spawner's registers: his panel, the element and his side, of which his init keeps the height.)
function thunman.spawn(controller: Object, user: Object, damage: number): Object?
    local x, y = controller.panel_x, controller.panel_y
    local o = battle.spawn(thunman.kind, Vec3.new(x + y * 0x100, controller.element, user.alliance))
    if not o then
        return nil
    end
    o.panel_x, o.panel_y = x, y
    o.alliance = user.alliance
    o:set_damage_word(damage)
    local s = o.state :: State
    s.controller = controller
    s.level = (controller.state :: NaviChipControllerState).level
    return o
end

return thunman
