--!strict
-- KnightMan, the navi of EXE5's KnightMan chips (EXE5's actor object #0x06, 0x080B9C28; EXE5's own, built on EXE6's
-- navi template), brought by their navi chip controller, wearing his ball and chain (EXE5's body overlay row 1). He
-- drops from 160 pixels up, falling faster each tick; off a solid panel he goes in smoke as he lands, else he
-- lands with a shake (a sound), stands 40 ticks, and swings (animation 18, a sound) his wrecking ball (./ball)
-- around himself once, hiding the one he wears, for 58 ticks; then he stands 30, crouches (animation 4) 4 and
-- flies up and away; the controller moves on.

local ball = require("./ball")
local navi_chips = require("@exelib/navi_chips/navi")
local effects = require("../../lib/effects")
local statuses = require("../../rules/status")

export type State = {
    -- The controller waiting for him (the flag it gave him, +0x54).
    controller: Object?,
    -- His chip's palette (the original's by the chip's level: DS's 4 gives 3).
    palette: number,
}

local knightman = {}

local SPRITE = asset.sprite("knightman")
local SOUND_LAND = asset.sound("land")
local SOUND_SWING = asset.sound("knight-swing")
-- What he wears (0x0800EE7A: EXE5's body overlay row 1, its sprite 10-47, in front at every animation), stepping
-- even while paused.
local CHAIN = new.identity {
    class = "navi",
    parts = { body = { sprite = asset.sprite("knightman-ball"), anims = 20 } },
}
-- Where he drops from and flies off to, how fast he falls faster, and how fast he rises.
local HEIGHT = 0xA0_0000
local GRAVITY = -0x4000
local RISE = 0x8_0000
-- His swing (the chips' Param2 to Param4: 8 a tick, one turn, confusing).
local SWING: KnightBallSwing = { speed = 8, turns = 1, status = statuses.confuse_480, look = ball.knightman }

-- He goes: the controller moves on, his ball and chain come off.
local function leave(me: Object)
    local controller = (me.state :: State).controller
    if controller then
        navi_chip.navi_left(controller)
    end
    me:remove_parts(CHAIN)
    -- (0x080B9E60: the state byte alone; his action and phase stay.)
    me.lifecycle = "destroy"
end

-- Action 0 (0x080B9CD4): shown, he falls; landed off a solid panel he goes in smoke (32 pixels up); else he lands,
-- the camera shaking, with a sound.
local function drop(me: Object)
    if me.phase_init == 0 then
        me.visible = true
        me.vel = Vec3.new(me.vel.x, me.vel.y, 0)
        me.phase_init = 4
    end
    local vz = int.i32(me.vel.z + GRAVITY)
    me.vel = Vec3.new(me.vel.x, me.vel.y, vz)
    local p = me.pos
    local z = int.i32(p.z + vz)
    me.pos = Vec3.new(p.x, p.y, z)
    if z > 0 then
        return
    end
    if not field.solid(me.panel_x, me.panel_y) then
        leave(me)
        local q = me.pos
        battle.effect(Vec3.new(q.x, q.y, int.i32(q.z + 0x20_0000)), effects.smoke)
        return
    end
    battle.shake_camera(2, 0xF)
    battle.play_sound(SOUND_LAND)
    me.pos = Vec3.new(p.x, p.y, 0)
    me:set_action(4)
end

-- Action 4 (0x080B9D4A): standing (animation 0), 40 ticks.
local function stand(me: Object)
    if me.phase_init == 0 then
        me.anim = 0
        me.timer = 0x28
        me.phase_init = 4
    end
    local left = me.timer - 1
    me.timer = int.u16(left)
    if left <= 0 then
        me:set_action(8)
    end
end

-- His ball and chain's overlay, held shown or hidden (`sub_80C44E4`, `sub_80C44FA`), or let go (`sub_80C4510`).
local function hold_chain(me: Object, shown: boolean?)
    local overlay = me.related2
    if not overlay then
        return
    end
    if shown == nil then
        overlay.phase_init = 0
        return
    end
    overlay.phase_init = 1
    overlay.visible = shown
end

-- Swing phase 0 (0x080B9D8C): his swing (animation 18, a sound, the ball he wears hidden): 58 ticks; then he stands
-- (animation 0), it shown again and let go.
local function swing(me: Object)
    if me.phase_init == 0 then
        me:set_animation(0x12)
        me.timer = 0x3A
        hold_chain(me, false)
        battle.play_sound(SOUND_SWING)
        ball.swing(me, bit32.bor(me.damage, bit32.lshift(me.stamina, 16)), SWING)
        me.phase_init = 4
        return
    end
    local left = me.timer - 1
    me.timer = int.u16(left)
    if left > 0 then
        return
    end
    me:set_animation(0)
    hold_chain(me, true)
    hold_chain(me, nil)
    me.phase, me.phase_init = 4, 0
end

-- Swing phase 4 (0x080B9DE0): 30 ticks.
local function rest(me: Object)
    if me.phase_init == 0 then
        me.timer = 0x1E
        me.phase_init = 4
    end
    local left = me.timer - 1
    me.timer = int.u16(left)
    if left <= 0 then
        me:set_action(0xC)
    end
end

local SWINGING = { [0] = swing, [4] = rest }

-- Action 8 (0x080B9D70): his swing, by its phase.
local function attack(me: Object)
    local phase = SWINGING[me.phase]
    if not phase then
        error(string.format("KnightMan's swing: phase %#x reads past its table (0x080B9D84)", me.phase))
    end
    phase(me)
end

-- Leaving phase 0 (0x080B9E1C): he crouches (animation 4), 4 ticks.
local function crouch(me: Object)
    if me.phase_init == 0 then
        me.anim = 4
        me.timer = 4
        me.phase_init = 4
    end
    local left = me.timer - 1
    me.timer = int.u16(left)
    if left <= 0 then
        me.phase, me.phase_init = 4, 0
    end
end

-- Leaving phase 4 (0x080B9E3E): he rises 8 pixels a tick; at 160 he goes.
local function rise(me: Object)
    local p = me.pos
    local z = int.i32(p.z + RISE)
    me.pos = Vec3.new(p.x, p.y, z)
    if z >= HEIGHT then
        leave(me)
    end
end

local LEAVING = { [0] = crouch, [4] = rise }

-- Action 0xC (0x080B9E02): his leaving, by its phase.
local function go(me: Object)
    local phase = LEAVING[me.phase]
    if not phase then
        error(string.format("KnightMan's leaving: phase %#x reads past its table (0x080B9E14)", me.phase))
    end
    phase(me)
end

local ACTIONS = { [0] = drop, [4] = stand, [8] = attack, [0xC] = go }

-- 0x080B9C4C: his sprite (animation 4, five frames in, his shadow), DS's palette 3, facing his side's way; his ball
-- and chain on; on his panel, 160 pixels up.
local function init(me: Object)
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "ground"
    me.anim, me.anim_loaded = 4, 4
    sprite:set_animation(4)
    for _ = 1, 5 do
        sprite:step()
    end
    sprite.palette = (me.state :: State).palette
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    me:add_parts(CHAIN, 1)
    me:set_coordinates_from_panel()
    local p = me.pos
    me.pos = Vec3.new(p.x, p.y, HEIGHT)
    me:set_lifecycle("update")
end

knightman.kind = new.kind {
    id = "knightmn/navi",
    pool = "actor",
    state = { controller = "object", palette = "u8" },
    -- 0x080B9C28.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
        elseif l == "update" then
            local action = ACTIONS[me.action]
            if not action then
                error(string.format("KnightMan action %#x reads past his table (0x080B9CC4)", me.action))
            end
            action(me)
        else
            me:free()
        end
        me:update_sprite_while_dimmed()
    end,
}

-- 0x080B9E64: KnightMan for `user`, reporting to `controller` (the summon table's entry 5), in his chip's `palette`.
function knightman.summon(palette: number): (user: Object, controller: Object, spec: NaviChipSpec) -> Object?
    return function(user: Object, controller: Object, spec: NaviChipSpec): Object?
        local o = navi_chips.spawn(knightman.kind, user, spec)
        if not o then
            return nil
        end
        local s = o.state :: State
        s.controller = controller
        s.palette = palette
        return o
    end
end


return knightman
