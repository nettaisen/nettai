--!strict
-- MetalMan, the navi of EXE4's MetalMan chips (EXE4's actor object #0x40, 0x080C7D44: EXE4's own), and their
-- controller, which brings him, with his fist (the follow effect's look 5, objects/follow_effect). He fades in on his
-- user's panel; while the battle is dimmed his user aims his punch: with A held, the d-pad moves a target over the
-- field (highlighted), from his own panel; A let go fixes it. 22 ticks after he came he blinks to the target's column
-- (unless it is his own panel), 3 ticks out, 3 in, a hit of no damage on the target; then, unless that is the field's
-- far edge or holds a body of his side, he winds up (40 ticks) and punches the panel ahead (60 ticks; the hit at 56
-- left: it breaks a solid panel, with a shake, a flash and a sound); else he waits 61 ticks. Then he goes.

local collision = require("../../rules/collision")
local follow_effect = require("../../objects/follow_effect")
local navi_chips = require("../../lib/navi_chips")
local regions = require("@exelib/regions")
local sparks = require("../../lib/sparks")

local metalman = {}

export type State = {
    -- The controller waiting for him (the flag it gave him, +0x64).
    controller: Object?,
    -- The chip's level (its Param1: 0, 3 for SP, 4 for DS): his palette.
    level: number,
    -- The gray of his color shader, fading in (+0x19).
    gray: number,
    -- His aim (Param2): 0 not begun, 1 A held, 2 fixed; his target (Param3, Param4).
    aim: number,
    target_x: number,
    target_y: number,
    -- His fist (+0x68).
    fist: Object?,
}

local SPRITE = asset.sprite("metalman")
local SOUND_APPEAR = asset.sound("appear")
local SOUND_WIND_UP = asset.sound("metal-fist")
local SOUND_PUNCH = asset.sound("metal-punch")
-- The punch's flash on a solid panel (effect row 0x47).
local FLASH = new.effect { sprite = asset.sprite("strike-flash"), anim = 0, palette = 0 }
-- His palettes a variant (0x0800B9B1's entry 6).
local PALETTES = 2
local FADE_GRAY = 0x1F
-- A body of each side (0x080C7F3C): a target that holds one of his own side stops his punch.
local OWN_BODY = { [0] = 0x0400_0000, 0x0800_0000 }

local function st(me: Object): State
    return me.state :: State
end

-- 0x080C8002: while the battle is dimmed (but under event flag 0x1187, which no netbattle sets), his side's navi aims
-- him: with A held the d-pad moves the target (up and down by rows, left and right by the side's view), highlighted;
-- A let go after it was held fixes it.
local function aim(me: Object)
    local s = st(me)
    if s.aim == 2 then
        return
    end
    local navi = battle.player(me.alliance) :: Object
    if not navi:dimmed_held("a") then
        if s.aim ~= 0 then
            s.aim = 2
        end
        return
    end
    -- A column toward his side's back (`-1`) or the other side.
    local function step(toward: number)
        local x = s.target_x + toward * me.facing
        if x >= 1 and x <= 6 then
            s.target_x = x
        end
    end
    if navi:dimmed_pressed("up") then
        if s.target_y > 1 then
            s.target_y -= 1
        end
    elseif navi:dimmed_pressed("down") then
        if s.target_y < 3 then
            s.target_y += 1
        end
    elseif navi:dimmed_pressed("left") then
        step(-1)
    elseif navi:dimmed_pressed("right") then
        step(1)
    end
    field.highlight(s.target_x, s.target_y)
    s.aim = 1
end

-- Action 0 (0x080C7E08): he fades in from gray 31, 2 a tick, flickering, his user aiming; then the attack.
local function fade_in(me: Object)
    local s = st(me)
    if me.phase_init == 0 then
        s.gray = FADE_GRAY
        battle.play_sound(SOUND_APPEAR)
        me.timer = 0
        me.phase_init = 4
        return
    end
    aim(me)
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
    me:set_action(4)
end

-- Action 4, phase 0 (0x080C7E8C): standing (animation 0), his user aiming, 22 ticks; then to the target, or, at his own
-- panel, the punch.
local function stand(me: Object)
    local s = st(me)
    if me.phase_init == 0 then
        me.anim = 0
        me.timer = 0x16
        me.phase_init = 4
    end
    aim(me)
    local left = me.timer - 1
    me.timer = int.u16(left)
    if left > 0 then
        return
    end
    local here = s.target_x == me.panel_x and s.target_y == me.panel_y
    me.phase, me.phase_init = if here then 0xC else 4, 0
end

-- Action 4, phase 4 (0x080C7EBC): he blinks out (animation 4), 3 ticks.
local function blink_out(me: Object)
    if me.phase_init == 0 then
        me.anim = 4
        me.timer = 3
        me.phase_init = 4
        return
    end
    local left = me.timer - 1
    me.timer = int.u16(left)
    if left <= 0 then
        me.phase, me.phase_init = 8, 0
    end
end

-- 0x080C8088: a hit of no damage on the target (row 0x27's, hitting navis; running while dimmed).
local function touch(me: Object, s: State)
    local box = battle.hitbox(me, {
        panel_x = s.target_x,
        panel_y = s.target_y,
        element = 0,
        z = 0,
        region = regions.single,
        hit_effect = nil,
        target = collision.hits_navis,
        self_type = collision.collision_27,
        damage = 0,
        stamina = 0,
        hit_mod = 0,
    })
    if box then
        box.run_while_dimmed = true
    end
end

-- Action 4, phase 8 (0x080C7EE0): he blinks in on the target (animation 3), 3 ticks; the touch; then the punch, unless
-- the target is the field's far edge or holds a body of his side: then a wait.
local function blink_in(me: Object)
    local s = st(me)
    if me.phase_init == 0 then
        me.anim = 3
        me.panel_x, me.panel_y = s.target_x, s.target_y
        me:set_coordinates_from_panel()
        me.timer = 3
        me.phase_init = 4
        return
    end
    local left = me.timer - 1
    me.timer = int.u16(left)
    if left > 0 then
        return
    end
    touch(me, s)
    local far = if me.alliance == 0 then 6 else 1
    if s.target_x == far or not field.check(s.target_x, s.target_y, 0, OWN_BODY[me.alliance]) then
        me.phase, me.phase_init = 0x14, 0
        return
    end
    me.phase, me.phase_init = 0xC, 0
end

-- Action 4, phase 0xC (0x080C7F44): the wind-up (animation 15) with its sound, 40 ticks.
local function wind_up(me: Object)
    if me.phase_init == 0 then
        me.anim = 15
        battle.play_sound(SOUND_WIND_UP)
        me.timer = 0x28
        me.phase_init = 4
        return
    end
    local left = me.timer - 1
    me.timer = int.u16(left)
    if left <= 0 then
        me.phase, me.phase_init = 0x10, 0
    end
end

-- 0x080C80A4: the punch on the panel ahead of the target (its target moves there): a breaking hit (row 21's, the
-- breaking spark); a solid panel breaks (`object_breakPanel_dup2`), with a shake, a flash and its sound.
local function punch(me: Object, s: State)
    s.target_x += me.facing
    local x, y = s.target_x, s.target_y
    local box = battle.hitbox(me, {
        panel_x = x,
        panel_y = y,
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
    if field.solid(x, y) then
        field.break_panel(x, y)
        battle.shake_camera(1, 0x28)
        local cx, cy = field.center(x, y)
        battle.effect(Vec3.new(cx, cy, 0), FLASH, me.alliance)
        battle.play_sound(SOUND_PUNCH)
    end
end

-- Action 4, phase 0x10 (0x080C7F6E): the punch (animation 10), 60 ticks, the hit at 56 left; then he goes.
local function strike(me: Object)
    if me.phase_init == 0 then
        me.anim = 10
        me.timer = 0x3C
        me.phase_init = 4
        return
    end
    if me.timer == 0x38 then
        punch(me, st(me))
    end
    local left = me.timer - 1
    me.timer = int.u16(left)
    if left <= 0 then
        me:set_action(8)
    end
end

-- Action 4, phase 0x14 (0x080C7FA0): 60 ticks; then he goes.
local function wait(me: Object)
    if me.phase_init == 0 then
        me.timer = 0x3C
        me.phase_init = 4
        return
    end
    local left = me.timer - 1
    me.timer = int.u16(left)
    if left <= 0 then
        me:set_action(8)
    end
end

-- Action 8 (0x080C7FBE): he goes (animation 4 from its first frame); 5 ticks, then out of sight.
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

local ATTACK: { [number]: (Object) -> () } = {
    [0] = stand,
    [4] = blink_out,
    [8] = blink_in,
    [0xC] = wind_up,
    [0x10] = strike,
    [0x14] = wait,
}

-- 0x080C7DC8: the action; the sprite, while dimmed too; his color shader from the gray (0x080C80F0).
local function tick(me: Object)
    local a = me.action
    if a == 0 then
        if me.phase ~= 0 then
            error(string.format("MetalMan action 0 phase %#x reads past its table (0x080C7E04)", me.phase))
        end
        fade_in(me)
    elseif a == 4 then
        local f = ATTACK[me.phase]
        if not f then
            error(string.format("MetalMan action 4 phase %#x reads past its table (0x080C7E74)", me.phase))
        end
        f(me)
    elseif a == 8 then
        leave(me)
    else
        error(string.format("MetalMan action %#x reads past his table (0x080C7DE0)", a))
    end
    me:update_sprite_while_dimmed()
    me.sprite.color_shader = st(me).gray * 0x421
end

-- 0x080C7D64: on his panel with his shadow, facing his side's way, his palette by the level, faded out; his fist (the
-- follow effect's look 5, a pixel above him, running while paused until the fight is on); his target his panel; his
-- height the spawner's.
local function init(me: Object)
    local s = st(me)
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
    -- 0x080C79A0.
    local p = me.pos
    local fist = follow_effect.spawn(me, Vec3.new(p.x, p.y - 0x10000, p.z), follow_effect.looks.metal_fist, me.alliance)
    if fist then
        fist.run_while_paused = true
    end
    s.fist = fist
    s.target_x, s.target_y = me.panel_x, me.panel_y
    s.aim = 0
    me:set_lifecycle("update")
    tick(me)
end

metalman.kind = new.kind {
    id = "metalman/navi",
    pool = "actor",
    state = {
        controller = "object",
        level = "u8",
        gray = "u8",
        aim = "u8",
        target_x = "u8",
        target_y = "u8",
        fist = "object",
    },
    -- 0x080C7D44.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
        elseif l == "update" then
            tick(me)
        else
            -- 0x080C7FEA: the controller moves on, his fist goes, and he is freed.
            local s = st(me)
            if s.controller then
                navi_chip.navi_left(s.controller)
            end
            if s.fist then
                follow_effect.stop(s.fist)
            end
            me:free()
        end
    end,
}

-- 0x080C8102: MetalMan for the controller's user, with the damage plus the bonus, of the chip's level. (His position is
-- the spawner's registers: his panel, the element and his side, of which his init keeps the height.)
function metalman.spawn(controller: Object, user: Object, damage: number): Object?
    local x, y = controller.panel_x, controller.panel_y
    local o = battle.spawn(metalman.kind, Vec3.new(x + y * 0x100, controller.element, user.alliance))
    if not o then
        return nil
    end
    o.panel_x, o.panel_y = x, y
    o.alliance = user.alliance
    o:set_damage_word(damage)
    local s = st(o)
    s.controller = controller
    s.level = (controller.state :: NaviChipControllerState).level
    return o
end

-- His chips' navi chip controller (EXE4's effect object #0x42, 0x080E6A18, which the original shares among six navis'
-- chips: compat/kinds.toml maps each one's to it): the user warps out, MetalMan, the user warps back in.
return navi_chips.controller {
    id = "metalman/controller",
    name = "MetalMan",
    spawn = metalman.spawn,
    steps = navi_chips.SHARED,
}
