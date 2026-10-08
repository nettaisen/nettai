--!strict
-- Roll as the RollAro chips bring her (EXE4's actor object #0x41, 0x080C8120: EXE4's own, on the template of the navi
-- chips' fading navis): on her user's panel, she fades in with a sparkle (the effect table's row 0x2E) and the
-- appearing sound, flickering; 15 ticks later she draws her bow (animation 7) and 10 ticks on shoots an arrow (the
-- flying shot's row 15) from 8 pixels ahead of her; 6 ticks on she lowers it for 30; then she leaves (her
-- animation 4, 5 ticks) and rises out of sight, a sparkle every 5 ticks, until 160 pixels up. Gone, she lets her
-- controller go on.

local collision = require("../../rules/collision")
local flying_shot = require("../../objects/flying_shot")
local panels = require("@exelib/panels")
local sparks = require("../../lib/sparks")

local roll = {}

export type State = {
    -- The controller waiting for her (the flag it gave her, +0x64).
    controller: Object?,
    -- The gray of her color shader, fading in (+0x19).
    gray: number,
}

local SPRITE = asset.sprite("roll")
local APPEAR_SOUND = asset.sound("appear")
local ARROW_SOUND = asset.sound("roll-arrow")
-- The effect table's row 0x2E: a sparkle (10-17).
local SPARKLE = new.effect { sprite = asset.sprite("sparkle"), anim = 0, palette = 0 }
-- Her palette (0x0800B950 by navi 1 and the chip's level, 0).
local PALETTE = 0
local ANIM_STAND, ANIM_LEAVE, ANIM_BOW = 0, 4, 7
local FADE_GRAY = 0x1F
-- The sparkle as she appears, 20 pixels over her; her rise, 15 pixels a tick, until 160 pixels up.
local SPARKLE_Z, RISE, GONE_Z = 20 * 0x10000, 15 * 0x10000, 0xA0

local APPEAR, SHOOT, LEAVE = 0x0, 0x4, 0x8

-- Her arrow, the flying shot's row 15 (0x080CE6A0): RollSoul's (row 0) at 7 pixels a tick: a chip destruction shot at
-- navis, hit modifier 3, spark 9, the arrow (0c-10, animation 0, drawn with its sprite), eight panel centers.
local ARROW = flying_shot.variant {
    is = collision.chip_destruction,
    hit_mod = 3,
    element = 0,
    spark = sparks.chip_destruction,
    sprite = asset.sprite("roll-arrow"),
    range = 8,
    speed = 7 * 0x10000,
}

-- Count the timer down: true while it is still above 0.
local function waits(me: Object): boolean
    local left = me.timer - 1
    me.timer = int.u16(left)
    return left > 0
end

-- Action 0, phase 0 (0x080C81D8): a sparkle over her, the sound; she fades in from gray 31, 2 a tick, flickering.
local function fade_in(me: Object)
    local s = me.state :: State
    if me.phase_init == 0 then
        local p = me.pos
        battle.effect(Vec3.new(p.x, p.y, p.z + SPARKLE_Z), SPARKLE)
        s.gray = FADE_GRAY
        battle.play_sound(APPEAR_SOUND)
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

-- Action 0, phase 4 (0x080C823A): 15 ticks; then the shot.
local function ready(me: Object)
    if me.phase_init == 0 then
        me.timer = 0xF
        me.phase_init = 4
        return
    end
    if waits(me) then
        return
    end
    me:set_action(SHOOT)
end

-- Action 4, phase 0 (0x080C827C): the bow drawn (animation 7); 10 ticks.
local function draw(me: Object)
    if me.phase_init == 0 then
        me.timer = 0xA
        me.anim = ANIM_BOW
        me.phase_init = 4
    end
    if waits(me) then
        return
    end
    me.phase, me.phase_init = 4, 0
end

-- Action 4, phase 4 (0x080C829E): the bow again from its first frame and the arrow, 8 pixels ahead of her, a pixel
-- up, 36 high, running while dimmed, with its sound; 6 ticks.
local function shoot(me: Object)
    if me.phase_init == 0 then
        me.timer = 6
        me.anim, me.anim_loaded = ANIM_BOW, 0xFF
        local p = me.pos
        local x = int.i32(p.x + panels.side_direction(me.alliance) * 8 * 0x10000)
        local y = int.i32(bit32.band(p.y, 0xFFFF0000) - 0x10000)
        local damage = bit32.bor(me.damage, bit32.lshift(me.stamina, 16))
        local o = flying_shot.launch(me, Vec3.new(x, y, 36 * 0x10000), ARROW, damage)
        if o then
            o.run_while_dimmed = true
        end
        battle.play_sound(ARROW_SOUND)
        me.phase_init = 4
    end
    if waits(me) then
        return
    end
    me.phase, me.phase_init = 8, 0
end

-- Action 4, phase 8 (0x080C82F4): the bow down (animation 0); 30 ticks; then she leaves.
local function lower(me: Object)
    if me.phase_init == 0 then
        me.timer = 0x1E
        me.anim = ANIM_STAND
        me.phase_init = 4
    end
    if waits(me) then
        return
    end
    me:set_action(LEAVE)
end

-- Action 8, phase 0 (0x080C8334): her leaving (animation 4 from its first frame); 5 ticks; then out of sight.
local function leave(me: Object)
    if me.phase_init == 0 then
        me.anim, me.anim_loaded = ANIM_LEAVE, 0
        me.timer = 5
        me.phase_init = 4
        return
    end
    if waits(me) then
        return
    end
    me.visible = false
    me.phase, me.phase_init = 4, 0
end

-- Action 8, phase 4 (0x080C8360): she rises 15 pixels a tick (from the tick after the first), a sparkle near her every
-- 5 ticks (0x080C83BE), and goes 160 pixels up.
local function rise(me: Object)
    if me.phase_init == 0 then
        me.timer = 4
        local v = me.vel
        me.vel = Vec3.new(v.x, v.y, RISE)
        me.phase_init = 4
    else
        local p = me.pos
        local z = int.i32(p.z + me.vel.z)
        me.pos = Vec3.new(p.x, p.y, z)
        if int.asr(z, 16) >= GONE_Z then
            me:set_lifecycle("destroy")
            return
        end
    end
    if waits(me) then
        return
    end
    me.timer = 5
    battle.effect(battle.jitter(0x10, me.pos), SPARKLE)
end

local ACTIONS: { [number]: { [number]: (Object) -> () } } = {
    [APPEAR] = { [0] = fade_in, [4] = ready },
    [SHOOT] = { [0] = draw, [4] = shoot, [8] = lower },
    [LEAVE] = { [0] = leave, [4] = rise },
}

-- 0x080C8194: the action; the sprite; her color shader from the gray (0x080C83AC).
local function tick(me: Object)
    local phases = ACTIONS[me.action]
    if not phases then
        error(string.format("RollAro's Roll: action %#x reads past her table (0x080C81AC)", me.action))
    end
    local f = phases[me.phase]
    if not f then
        error(string.format("RollAro's Roll: action %#x phase %#x reads past its table", me.action, me.phase))
    end
    f(me)
    me:update_sprite()
    me.sprite.color_shader = (me.state :: State).gray * 0x421
end

-- 0x080C8140: shown, with her shadow, on her panel, facing her side's way, in her palette, faded out (her height the
-- spawner's); her first tick at once.
local function init(me: Object)
    me.visible = true
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "ground"
    me.anim, me.anim_loaded = ANIM_STAND, 0
    sprite:set_animation(ANIM_STAND)
    me:set_coordinates_from_panel()
    sprite:set_flip(me.alliance)
    sprite.palette = PALETTE
    sprite.color_shader = 0x7FFF
    me:set_lifecycle("update")
    tick(me)
end

roll.kind = new.kind {
    id = "rollaro/roll",
    pool = "actor",
    state = { controller = "object", gray = "u8" },
    -- 0x080C8120.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
        elseif l == "update" then
            tick(me)
        else
            -- 0x080C839E: the controller moves on, and she is freed.
            local controller = (me.state :: State).controller
            if controller then
                navi_chip.navi_left(controller)
            end
            me:free()
        end
    end,
}

-- 0x080C83D4: Roll on the controller's panel, of its side, with the damage word (bonus included); the controller
-- waits for her. (Her position is the spawner's registers: her panel and her side, of which her init keeps the
-- height.)
function roll.spawn(controller: Object, _user: Object, damage: number): Object?
    local x, y = controller.panel_x, controller.panel_y
    local o = battle.spawn(roll.kind, Vec3.new(x, y, controller.alliance))
    if not o then
        return nil
    end
    o.panel_x, o.panel_y = x, y
    o.alliance = controller.alliance
    o:set_damage_word(damage)
    ;(o.state :: State).controller = controller
    return o
end

return roll
