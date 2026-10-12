--!strict
-- DarkNeo's image of its user (EXE4's attack object #0xA1, 0x080E1028; it has no collision), which DarkNeo's Bass
-- (./navi) leaves on his panel: the user's navi's sprite (by its navi and soul, 0x0800B90A: the engine's `load_like`)
-- in the palette 0x0800B990 gives its navi (MegaMan's 1), fading in over 16 ticks with the appearing's sound; what
-- its navi's form wears put on (0x0800B812); 20 ticks; then it raises the bug charger (attachment row 0x2E, 0c-43,
-- with sound 0x139) and fires nine waves (./wave) from the panel ahead, 11 ticks apart, in its row, the row above and
-- the row below in turn, each with a small shake and a spark ahead of it (effect row 0x6C, jittered); it lowers the
-- charger and waits for Bass, who tells it when he leaves; then it blinks out over 31 ticks, what it wore taken off
-- (0x0800B5DA). It runs while dimmed.

local attachment = require("../../objects/attachment")
local wave = require("./wave")

local image = {}

-- The navi's number: its palette and what it holds go by it (NaviStats +0x23, MegaMan's 0).
-- (0x0800B9B1's entry for the navi, times 1.)
local PALETTE = 1
-- The charger's animation as it is raised and once it has fired (0x080E119E, 0x080E1222: 1 and 2 past twice the
-- sprite's category the image was given, a navi's 0).
local RAISED_ANIM, FIRED_ANIM = 1, 2

-- Attachment row 0x2E (0x080CD524): 0c-43 in palette 0, no lift.
local CHARGER = attachment.look { sprite = asset.sprite("bug-charger"), palette = 0 }
-- Effect row 0x6C (0x080E1698): 14-05, animation 10.
local SPARK = new.effect { sprite = asset.sprite("hit-sparks"), anim = 10, palette = 0 }
local APPEAR = asset.sound("appear")
local RAISE = asset.sound("darkneo-charger")

local FADE, WAIT = 0x0F, 0x14
local ATTACK_ANIM = 13
local WAVES, BETWEEN = 8, 0x0A
local WAVE_HEIGHT = 0x10_0000
-- The waves' rows from its own, by the count left (0x080E1258).
local ROWS = { [0] = 1, -1, 0, 1, -1, 0, 1, -1, 0 }
local SPARK_AHEAD, SPARK_JITTER = 0x28_0000, 7
local LEAVE, LAST_BLINKS = 0x1E, 0x0A

-- Its actions: fading in, the wait, waiting for Bass, the attack, leaving.
local FADING, WAITING, STANDING, ATTACKING, LEAVING = 0x0, 0x4, 0x8, 0xC, 0x10

local function damage_word(me: Object): number
    return bit32.bor(me.damage, bit32.lshift(me.stamina, 16))
end

-- 0x080E10F4: a sixteenth more a tick; then solid, wearing what its navi's form wears.
local function fade_in(me: Object)
    local shown = me.timer2 + 1
    me.timer2 = shown
    me.sprite.alpha = shown
    local t = me.timer - 1
    me.timer = int.u16(t)
    if t >= 0 then
        return
    end
    me.sprite.alpha = nil
    me:put_on_form_overlay(battle.navi(me.alliance).form)
    me.timer = WAIT
    me:set_action(WAITING)
end

-- 0x080E1122.
local function wait(me: Object)
    local t = me.timer - 1
    me.timer = int.u16(t)
    if t == 0 then
        me:set_action(ATTACKING)
    end
end

-- A wave from the panel ahead, in the row its count says; the shake; the spark 40 pixels ahead of it.
local function fire(me: Object)
    local y = bit32.band(me.panel_y + ROWS[me.timer2], 0xFF)
    wave.spawn(me, me.panel_x + me.facing, y, WAVE_HEIGHT, damage_word(me))
    battle.shake_camera(1, 5)
    local p = me.pos
    local pos = battle.jitter(SPARK_JITTER, Vec3.new(p.x + me.facing * SPARK_AHEAD, p.y, p.z))
    battle.effect(pos, SPARK, me.alliance)
end

-- 0x080E118A: the charger up (11 ticks), the nine waves, 11 ticks more, the charger down.
local function attack(me: Object)
    local init = me.phase_init
    if init == 2 then
        local t = me.timer - 1
        me.timer = int.u16(t)
        if t >= 0 then
            return
        end
        -- 0x080CD708: the charger goes.
        local charger = me.related1
        if charger then
            charger:set_lifecycle("destroy")
        end
        me.anim = 0
        me:set_action(STANDING)
        return
    end
    if init ~= 1 then
        me.anim = ATTACK_ANIM
        attachment.attach(me, CHARGER, "related", RAISED_ANIM, true)
        me.timer2 = WAVES
        battle.play_sound(RAISE)
        me.timer = BETWEEN
        me.phase_init = 1
    end
    local t = me.timer - 1
    me.timer = int.u16(t)
    if t >= 0 then
        return
    end
    fire(me)
    me.timer = BETWEEN
    if me.timer2 == WAVES then
        local charger = me.related1
        if charger then
            charger.anim = FIRED_ANIM
        end
    end
    local left = me.timer2 - 1
    me.timer2 = int.u16(left)
    if left < 0 then
        me.phase_init = 2
    end
end

-- 0x080E1136: until Bass tells it he has left (its phase's init byte, which his leaving sets).
local function stand(me: Object)
    if me.phase_init ~= 0 then
        me:set_action(LEAVING)
    end
end

-- 0x080E1148: 31 ticks, hidden two ticks of every four over the last 11; what it wore off, and gone.
local function leave(me: Object)
    if me.phase_init == 0 then
        me.phase_init = 1
        me.timer = LEAVE
    end
    local t = me.timer
    me.visible = not (t <= LAST_BLINKS and not bit32.btest(t, 2))
    t -= 1
    me.timer = int.u16(t)
    if t >= 0 then
        return
    end
    if me.related2 then
        me:take_off_form_overlay(battle.navi(me.alliance).form)
    end
    me:set_lifecycle("destroy")
end

local ACTIONS = { [FADING] = fade_in, [WAITING] = wait, [STANDING] = stand, [ATTACKING] = attack, [LEAVING] = leave }

-- 0x080E10CE.
local function tick(me: Object)
    local action = ACTIONS[me.action]
    if not action then
        error(string.format("DarkNeo's image: action %#x reads past its table (0x080E10E0)", me.action))
    end
    action(me)
end

-- 0x080E104C: nothing off the field. Its navi's sprite with its shadow, shown, its animation 0, in its palette,
-- facing its side's way, on its panel; unseen yet (blended at none), with the appearing's sound.
local function init(me: Object)
    if not field.valid(me.panel_x, me.panel_y) then
        me:free()
        return
    end
    local navi = battle.player(me.alliance)
    if not navi then
        error("DarkNeo's image has no navi to look like (0x0800B90A's caller read its side's navi stats)")
    end
    local sprite = me.sprite
    sprite:load_like(navi)
    sprite.shadow = "ground"
    me.visible = true
    me.anim, me.anim_loaded = 0, 0
    sprite:set_animation(0)
    sprite.palette = PALETTE
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    me:set_coordinates_from_panel()
    me.timer = FADE
    me.timer2 = 0
    sprite.alpha = 0
    battle.play_sound(APPEAR)
    me:set_lifecycle("update")
end

image.kind = new.kind {
    id = "darkneo/image",
    pool = "attack",
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
        elseif l == "update" then
            tick(me)
        else
            me:free()
            return
        end
        me:update_sprite()
    end,
}

-- 0x080E1264: the image of `owner`'s side's navi on (x, y), null, with the damage word, running while dimmed. (Its
-- position is the spawner's registers, the row and two zeros, until its init places it; on the ground.)
function image.spawn(owner: Object, x: number, y: number, damage: number): Object?
    local o = battle.spawn(image.kind, Vec3.new(y, 0, 0))
    if o then
        o.panel_x, o.panel_y = x, y
        o.element = 0
        o:set_damage_word(damage)
        o.alliance = owner.alliance
        o.run_while_dimmed = true
    end
    return o
end

-- Bass has left (0x080CB36A: he sets its phase's init byte).
function image.released(o: Object)
    o.phase_init = 1
end

return image
