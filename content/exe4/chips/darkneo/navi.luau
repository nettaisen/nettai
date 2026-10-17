--!strict
-- DarkNeo's Bass (EXE4's actor object #0x51, 0x080CB320: EXE4's own; the Bass chip's Bass, #0x4F, as far as his
-- rise). He appears on his user's panel with his cape (a form overlay, his sheet 08-19, its animations 0x0E on) and
-- the appearing's sound, and rises for 101 ticks, half a pixel a tick over the last 81. Then:
--
-- - his arm goes up (animation 9, then 10) with the hole's sound, which sounds again every 33 ticks; an aura around him
--   (his sheet's animation 0x20, an attachment), his shadow gone; over 31 ticks the panel under him shows as a hole
--   four ticks of every eight, then is one (the Hole chip's, type 11), unless it had no floor when he came;
-- - his user's image appears there (./image), which fires its waves while he waits 141 ticks;
-- - he flies to the middle of the field (column 4 for side 0, 3 for side 1, row 2) in six ticks, coming down as he
--   goes, his shadow back, leaving four ghosts behind (./ghost: his body's and his cape's, 4 and 10 ticks after him),
--   with a swing's sound (0x127), and the aura goes;
-- - there, unless a body is in his way (then he leaves at once), he blasts the two columns ahead (animations 11 and
--   12, sound 0xFC, effect row 0x59 over the field): a hit on them from the middle row of the column past his
--   (region 0x11, piercing and breaking, thrown, hitting navis, hit modifier 3, no spark), the camera shaking 60
--   ticks; 61 ticks; a last shake, animation 13, 21 ticks.
--
-- He leaves in smoke (effect row 0x13), his cape with him; his controller moves on and the image goes. He runs while
-- dimmed.

local attachment = require("../../objects/attachment")
local collision = require("../../rules/collision")
local ghost = require("./ghost")
local image = require("./image")
local regions = require("@exelib/regions")

local navi = {}

type State = {
    -- The controller waiting for him (the original keeps its slot's address in his Z velocity).
    controller: Object?,
    -- His panel had no floor when he came (its flag 0x8000: his +0x60): it is left as it is.
    floorless: boolean,
    -- His aura (his +0x64) and his user's image (his +0x68).
    held: Object?,
    image: Object?,
}

local SPRITE = asset.sprite("bass")
local CAPE_ANIM_OFFSET = 0x0E
-- Attachment row 0x1E (0x080CD524): his sheet in palette 0, no lift; its animation 0x20, animating while dimmed.
local AURA = attachment.look { sprite = SPRITE, palette = 0 }
local AURA_ANIM = 0x20
-- Effect rows 0x13 (14-0d) and 0x59 (his sheet's animation 0x23).
local SMOKE = new.effect { sprite = asset.sprite("puff"), anim = 0, palette = 0 }
local BLAST = new.effect { sprite = SPRITE, anim = 0x23, palette = 0 }
local APPEAR = asset.sound("appear")
local HOLE_SOUND = asset.sound("panel-hole")
local FLY_SOUND = asset.sound("windrack")
local BLAST_SOUND = asset.sound("bass-blast")

local NO_FLOOR = 0x8000
local RISE, RISING = 0x64, 0x50
local HOLE_SOUND_EVERY, HOLE_TICKS = 0x20, 0x1E
local IMAGE_WAIT = 0x8C
local FLIGHT, FALL = 6, 0x6_AAAA
-- His ghosts' tints.
local NEAR_SHADER, FAR_SHADER = 0x1000, 0x2083
-- Where the blast's effect shows (the field's middle, 36 pixels down it, a pixel up) and its hit's height.
local BLAST_AT = Vec3.new(0, 0x24_0000, 0x1_0000)
local BLAST_HEIGHT = 0x10_0000
-- The panel he lands on must hold none of these, by his side (0x080CB640).
local IN_HIS_WAY = { [0] = 0x0780_0000, [1] = 0x0B80_0000 }

local RISE_ACTION, CAST, BLAST_ACTION = 0x0, 0x4, 0x8
local ARM_UP, HOLE, IMAGE, FLY = 0x0, 0x4, 0x8, 0xC

-- `sub_800E258`: the panel under a position (the BIOS divisions), as `side` has it
-- (field.panel_at: a line between columns is in the one toward the other side, for either side).
local function panel_at(x: number, y: number, side: number): (number, number)
    return field.panel_at(x, y, side)
end

-- Count the timer down: true once it fell below 0.
local function ran_out(me: Object): boolean
    local left = me.timer - 1
    me.timer = int.u16(left)
    return left < 0
end

-- 0x080CB418: 101 ticks, rising half a pixel a tick over the last 81.
local function rise(me: Object)
    if me.timer <= RISING then
        local p = me.pos
        me.pos = Vec3.new(p.x, p.y, p.z + 0x8000)
    end
    if ran_out(me) then
        me:set_action(CAST)
    end
end

-- 0x080CB464: his arm up; when it is, the aura, his shadow gone.
local function arm_up(me: Object)
    if me.phase_init == 0 then
        me.phase_init = 1
        me.anim = 9
    end
    if not bit32.btest(me.sprite.frame_flags, 0x80) then
        return
    end
    me.anim = 10
    battle.play_sound(HOLE_SOUND)
    me.timer2 = HOLE_SOUND_EVERY
    attachment.attach(me, AURA, "held", AURA_ANIM, true)
    local sprite = me.sprite
    sprite.hidden_parts = bit32.bor(sprite.hidden_parts, 0x8000_0000)
    me.timer = HOLE_TICKS
    me.phase, me.phase_init = HOLE, 0
end

-- The hole's sound every 33 ticks.
local function hum(me: Object)
    local left = me.timer2 - 1
    me.timer2 = int.u16(left)
    if left < 0 then
        me.timer2 = HOLE_SOUND_EVERY
        battle.play_sound(HOLE_SOUND)
    end
end

-- 0x080CB4B0: the panel under him turning to a hole; then his user's image there.
local function hole(me: Object)
    local s = me.state :: State
    local x, y = me.panel_x, me.panel_y
    if not bit32.btest(me.timer, 4) and not s.floorless then
        field.blink(x, y, "hole", me.alliance)
    end
    hum(me)
    if not ran_out(me) then
        return
    end
    if not s.floorless then
        field.set_type(x, y, "hole")
    end
    -- (0x0800B90A: the image's sprite is its side's navi's, by its navi and soul: the image's own `load_like`.)
    s.image = image.spawn(me, x, y, bit32.bor(me.damage, bit32.lshift(me.stamina, 16)))
    me.timer = IMAGE_WAIT
    me.phase, me.phase_init = IMAGE, 0
end

-- 0x080CB52E: 141 ticks for the image; then off to the middle, his ghosts behind him.
local function wait_for_image(me: Object)
    if not ran_out(me) then
        return
    end
    me.timer, me.timer2 = FLIGHT, FLIGHT
    me.vel = Vec3.new(4 - me.alliance - me.panel_x, 2 - me.panel_y, me.vel.z)
    ghost.spawn(me, 10, 4, NEAR_SHADER, FALL)
    ghost.spawn(me, 10 + CAPE_ANIM_OFFSET, 4, NEAR_SHADER, FALL)
    ghost.spawn(me, 10, 10, FAR_SHADER, FALL)
    ghost.spawn(me, 10 + CAPE_ANIM_OFFSET, 10, FAR_SHADER, FALL)
    local sprite = me.sprite
    sprite.hidden_parts = bit32.band(sprite.hidden_parts, 0x7FFF_FFFF)
    battle.play_sound(FLY_SOUND)
    me.phase, me.phase_init = FLY, 0
end

-- 0x080CB5D0: the flight; at its end the aura goes, and he blasts unless a body is in his way.
local function fly(me: Object)
    local x, y = ghost.position(me, ghost.flight(me.timer))
    me.pos = Vec3.new(x, y, int.i32(me.pos.z - FALL))
    local t = me.timer - 1
    me.timer = int.u16(t)
    if t ~= 0 then
        return
    end
    -- 0x080CD708: the aura goes.
    local aura = (me.state :: State).held
    if aura then
        aura:set_lifecycle("destroy")
    end
    local px, py = panel_at(me.pos.x, me.pos.y, me.alliance)
    if not field.check(px, py, 0, IN_HIS_WAY[me.alliance]) then
        me:set_lifecycle("destroy")
        return
    end
    me.anim = 11
    me:set_action(BLAST_ACTION)
end

local CAST_PHASES = { [ARM_UP] = arm_up, [HOLE] = hole, [IMAGE] = wait_for_image, [FLY] = fly }

-- 0x080CB440.
local function cast(me: Object)
    local f = CAST_PHASES[me.phase]
    if not f then
        error(string.format("DarkNeo's Bass: phase %#x reads past his table (0x080CB454)", me.phase))
    end
    f(me)
end

-- 0x080CB650: when his landing's animation ends, the blast; 61 ticks; the last shake and 21 ticks; then he leaves.
local function blast(me: Object)
    local init = me.phase_init
    if init == 0 then
        if not bit32.btest(me.sprite.frame_flags, 0x80) then
            return
        end
        me.phase_init = 1
        me.anim = 12
        battle.play_sound(BLAST_SOUND)
        battle.effect(BLAST_AT, BLAST, me.alliance)
        -- 0x080CD81E: the hit, running while dimmed, from the middle row of the column past his.
        local box = battle.hitbox(me, {
            panel_x = 5 - 3 * me.alliance,
            panel_y = 2,
            element = 0,
            z = BLAST_HEIGHT,
            region = regions.wide_long,
            -- None.
            hit_effect = nil,
            target = collision.hits_navis,
            self_type = collision.thrown_piercing_break,
            damage = me.damage,
            stamina = me.stamina,
            hit_mod = 3,
        })
        if box then
            box.run_while_dimmed = true
        end
        battle.shake_camera(2, 0x3C)
        me.timer = 0x3C
        init = 1
    end
    if init == 1 then
        if not ran_out(me) then
            return
        end
        battle.shake_camera(0, 0x14)
        me.anim = 13
        me.timer = 0x14
        me.phase_init = 2
    end
    if ran_out(me) then
        me:set_lifecycle("destroy")
    end
end

local ACTIONS = { [RISE_ACTION] = rise, [CAST] = cast, [BLAST_ACTION] = blast }

-- 0x080CB3E8: his action; gone if his panel is off the field.
local function tick(me: Object)
    local action = ACTIONS[me.action]
    if not action then
        error(string.format("DarkNeo's Bass: action %#x reads past his table (0x080CB40C)", me.action))
    end
    action(me)
    if not field.valid(me.panel_x, me.panel_y) then
        me:set_lifecycle("destroy")
    end
end

-- 0x080CB374: on his panel, with his shadow and his cape; whether the panel has a floor; then his first tick.
local function init(me: Object)
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "ground"
    me.visible = true
    me.anim, me.anim_loaded = 0, 0
    sprite:set_animation(0)
    sprite.palette = 0
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    me:set_coordinates_from_panel()
    battle.play_sound(APPEAR)
    me.related1 =
        battle.form_overlay(me, { sprite = SPRITE, stepping = "while_dimmed", anim_offset = CAPE_ANIM_OFFSET })
    local s = me.state :: State
    s.floorless = bit32.btest(field.flags(me.panel_x, me.panel_y), NO_FLOOR)
    me.timer = RISE
    me:set_lifecycle("update")
    tick(me)
end

-- 0x080CB344: smoke where he was, his cape off; his controller moves on, and the image is told.
local function leave(me: Object)
    local p = me.pos
    battle.effect(Vec3.new(p.x, p.y, p.z + 0x10_0000), SMOKE)
    local cape = me.related1
    if cape then
        cape:set_lifecycle("destroy")
    end
    local s = me.state :: State
    if s.controller then
        navi_chip.navi_left(s.controller)
    end
    if s.image then
        image.released(s.image)
    end
    me:free()
end

navi.kind = new.kind {
    id = "darkneo/navi",
    pool = "actor",
    state = { controller = "object", floorless = "bool", held = "object", image = "object" },
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
        elseif l == "update" then
            tick(me)
        else
            leave(me)
        end
        me:update_sprite_while_dimmed()
    end,
}

-- 0x080CB6E4: Bass on (x, y) for `controller`, of its side, with the element and the damage word. (His position is
-- the spawner's registers, the row, the element and 0, until his init places him; on the ground.)
function navi.summon(controller: Object, x: number, y: number, element: number, damage: number): Object?
    local o = battle.spawn(navi.kind, Vec3.new(y, element, 0))
    if not o then
        return nil
    end
    o.panel_x, o.panel_y = x, y
    o.element = element
    o.alliance = controller.alliance
    o:set_damage_word(damage)
    ;(o.state :: State).controller = controller
    return o
end

return navi
