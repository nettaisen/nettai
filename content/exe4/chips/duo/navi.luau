--!strict
-- Duo, as his chip brings him (EXE4's actor object #0x52, 0x080CB70C: EXE4's own). He takes his side's back two
-- columns:
--
-- - for 41 ticks their six panels blink away, four ticks of every eight (drawn only: the panels are as they were for
--   everything that stands or lands on them), and stay away while he is there;
-- - he comes in from behind the field's edge in the middle row (his sheet 08-18, his sound), 8 pixels a tick for 11
--   ticks, and stops with a thud (the camera shaking 30 ticks);
-- - 61 ticks later he throws 17 fists (./fist), one hand's and the other's by turns, 8 ticks apart: the first at a
--   random panel an enemy stands on, the others at random panels of the enemy's area around the enemies, never the
--   last one twice in a row (@exelib/metrknuk/targets: EXE6's and EXE5's MetrKnuk's searches are his); with no enemy
--   standing, or no such panel, no fist goes that time;
-- - 31 ticks after the last, the camera shakes again and he backs out the way he came, 21 ticks; then his panels are
--   drawn again and his controller moves on.
--
-- He runs while dimmed.

local fists = require("./fist")
local panels = require("@exelib/panels")
local targets = require("@exelib/metrknuk/targets")

local navi = {}

type State = {
    -- The controller waiting for him (the flag it gave him, his +0x4C).
    controller: Object?,
    -- His last target and the panels around the enemies (his +0x60 and +0x64: @exelib/metrknuk/targets').
    last: number,
    grid: { number },
}

local SPRITE = asset.sprite("duo")
local SOUND_ARRIVE = asset.sound("red-sun-dive")
local SOUND_STOP = asset.sound("metal-punch")

-- His fists' targets: an enemy's panel is any an enemy's body is on (0x080CBABC); with no enemy standing no fist goes
-- (0x080CB9C8).
local AIM: FistTargets = { body = targets.BODY, area_fallback = false }
-- His fists: 17 (his count runs from 16 down through 0), from 120 pixels up, of no element.
local PUNCHES, HEIGHT = 16, 0x78_0000
-- The ticks to the next fist, less one.
local BETWEEN = 6
-- His panels blink away for 41 ticks; he moves 8 pixels a tick for 11; 61 ticks before his fists, 31 after them, 21
-- backing out.
local BLINK, SPEED, GLIDE, WAIT, REST, LEAVE = 40, 0x8_0000, 10, 60, 30, 20

local ARRIVE, STAND, PUNCH, GO = 0x0, 0x4, 0x8, 0xC

-- Count the timer down: true once it fell below 0.
local function ran_out(me: Object): boolean
    local left = me.timer - 1
    me.timer = int.u16(left)
    return left < 0
end

-- 0x080CB992: his side's back two columns' six panels are drawn, or not (0x08009E5C, 0x08009E74).
local function show_panels(me: Object, shown: boolean)
    local x = me.alliance * 4 + 1
    for column = x, x + 1 do
        for y = 3, 1, -1 do
            field.set_visible(column, y, shown)
        end
    end
end

-- 0x080CB84C: 8 pixels on; when the count runs out he stops: the camera shakes, his sound, and he stands.
local function glide(me: Object)
    local p = me.pos
    me.pos = Vec3.new(int.i32(p.x + me.vel.x), p.y, p.z)
    if not ran_out(me) then
        return
    end
    me.timer = WAIT
    battle.shake_camera(1, 30)
    battle.play_sound(SOUND_STOP)
    me:set_action(STAND)
end

-- Action 0 (0x080CB780), by phase: 0 (0x080CB7A0) his panels blinking away; 4 (0x080CB7E4) he appears behind the
-- edge of the middle row (column 0, or 7 for side 1) and starts in; 8 his glide.
local function arrive(me: Object)
    local phase = me.phase
    if phase == 0 then
        if me.phase_init == 0 then
            me.phase_init = 1
            me.timer = BLINK
        end
        show_panels(me, bit32.btest(me.timer, 4))
        if ran_out(me) then
            me.phase, me.phase_init = 4, 0
        end
    elseif phase == 4 then
        local sprite = me.sprite
        sprite:load(SPRITE)
        sprite.shadow = "ground"
        me.visible = true
        me.anim, me.anim_loaded = 0, 0
        sprite:set_animation(0)
        sprite.palette = 0
        sprite:set_flip(bit32.bxor(me.alliance, me.flip))
        me.panel_x, me.panel_y = me.alliance * 7, 2
        me:set_coordinates_from_panel()
        battle.play_sound(SOUND_ARRIVE)
        me.vel = Vec3.new(me.facing * SPEED, me.vel.y, me.vel.z)
        me.timer = GLIDE
        -- His glide's first tick at once.
        glide(me)
        me.phase, me.phase_init = 8, 0
    elseif phase == 8 then
        glide(me)
    else
        error(string.format("Duo's arrival: phase %#x reads past its table (0x080CB794)", phase))
    end
end

-- Action 4 (0x080CB87A): 61 ticks; then his fists.
local function stand(me: Object)
    if ran_out(me) then
        me.timer2 = PUNCHES
        me:set_action(PUNCH)
    end
end

-- Action 8 (0x080CB892): phase 0 (0x080CB8AC) a fist at its target, if there is one, one hand's on an even count
-- and the other's on an odd; after the count's last, on; else 7 ticks (phase 4, 0x080CB8EC) to the next.
local function punch(me: Object)
    if me.phase == 4 then
        if ran_out(me) then
            me.phase, me.phase_init = 0, 0
        end
        return
    elseif me.phase ~= 0 then
        error(string.format("Duo's fists: phase %#x reads past their table (0x080CB8A4)", me.phase))
    end
    local s = me.state :: State
    local n = me.timer2
    -- 0x080CBA0C the first, 0x080CB9C2 the others.
    local target = if n == PUNCHES then targets.first(me.alliance, AIM) else targets.next(me.alliance, s, AIM)
    s.last = target
    local x = panels.x(target)
    if x ~= 0 then
        -- 0x080CB9B0: with his damage word.
        local fist = if bit32.btest(n, 1) then fists.odd else fists.even
        fist.spawn(me, x, panels.y(target), 0, HEIGHT, bit32.bor(me.damage, bit32.lshift(me.stamina, 16)))
    end
    me.timer2 = int.u16(n - 1)
    if n - 1 < 0 then
        me:set_action(GO)
        return
    end
    me.timer = BETWEEN
    me.phase, me.phase_init = 4, 0
end

-- 0x080CB94A: 8 pixels back; when the count runs out he is gone.
local function back_out(me: Object)
    local p = me.pos
    me.pos = Vec3.new(int.i32(p.x - me.vel.x), p.y, p.z)
    if ran_out(me) then
        me:set_lifecycle("destroy")
    end
end

-- Action 0xC (0x080CB8FC): phase 0 (0x080CB91C) 31 ticks, then the camera shakes and he starts back; phase 4 his
-- way out. (Phase 8, 0x080CB962, a count that ends nothing, nothing sets.)
local function go(me: Object)
    local phase = me.phase
    if phase == 0 then
        if me.phase_init == 0 then
            me.phase_init = 1
            me.timer = REST
        end
        if not ran_out(me) then
            return
        end
        battle.shake_camera(1, 20)
        me.timer = LEAVE
        me.phase, me.phase_init = 4, 0
        back_out(me)
    elseif phase == 4 then
        back_out(me)
    elseif phase == 8 then
        ran_out(me)
    else
        error(string.format("Duo's leaving: phase %#x reads past its table (0x080CB910)", phase))
    end
end

local ACTIONS: { [number]: (Object) -> () } = { [ARRIVE] = arrive, [STAND] = stand, [PUNCH] = punch, [GO] = go }

-- 0x080CB75E.
local function tick(me: Object)
    local f = ACTIONS[me.action]
    if not f then
        error(string.format("Duo's action %#x reads past his table (0x080CB770)", me.action))
    end
    f(me)
end

navi.kind = new.kind {
    id = "duo/navi",
    pool = "actor",
    state = { controller = "object", last = targets.LAST, grid = targets.GRID },
    -- 0x080CB70C: by his state, then his sprite, while dimmed too.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            -- 0x080CB74C: (his sheet's tiles made ready;) his first tick at once.
            me:set_lifecycle("update")
            tick(me)
        elseif l == "update" then
            tick(me)
        else
            -- 0x080CB730: his controller moves on; his panels are drawn again; he is freed.
            local controller = (me.state :: State).controller
            if controller then
                navi_chip.navi_left(controller)
            end
            show_panels(me, true)
            me:free()
            return
        end
        me:update_sprite_while_dimmed()
    end,
}

-- 0x080CB96E: Duo for `controller`, of its side, with the damage word. (His panel is the controller's until he
-- appears; his position the spawner's registers: the row, the element and 0.)
function navi.summon(controller: Object, damage: number): Object?
    local o = battle.spawn(navi.kind, Vec3.new(controller.panel_y, controller.element, 0))
    if not o then
        return nil
    end
    o.panel_x, o.panel_y = controller.panel_x, controller.panel_y
    o.element = controller.element
    o.alliance = controller.alliance
    o:set_damage_word(damage)
    ;(o.state :: State).controller = controller
    return o
end

return navi
