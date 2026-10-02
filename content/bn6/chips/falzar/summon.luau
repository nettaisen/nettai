--!strict
-- Falzar's summon (the original's actor object #0x31; the Japanese ROMs'
-- 0x080C4268 in the Falzar ROM, 0x080C5AC8 in the Gregar ROM; the US ROMs
-- point the slot at a placeholder; docs/engine/beast-chips.md): the cyber
-- beast, drawn in three pieces 36 pixels over its user. The pieces appear
-- (60 ticks), flash white (15), rise (30), and stay (120); the leading
-- piece brings the arrival's look as it appears, and while it stays it
-- cries and sends a whirlwind from the panel in front of its user
-- (./tornado), then tells the controller it is done. They then sink (10)
-- and go.

local arrival = require("../../lib/beast-chips/arrival")
local tornado = require("./tornado")

local summon = {}

export type State = {
    -- The piece that leads (its Param1).
    leads: boolean,
    -- The piece's first animation of four (its Param2).
    anims: number,
    -- The controller, whose flag byte (its Param1) the summon holds: set
    -- while it is out (+0x54).
    controller: Object?,
}

local SPRITE = asset.sprite("bn6:falzar-summon")
-- The arrival's look: animation 0x0C of its sprite.
local ARRIVAL_ANIM = 0x0C

local SOUND_APPEAR = asset.sound("bn6:log-in")
local SOUND_FLASH = asset.sound("bn6:bug")
local SOUND_CRY = asset.sound("bn6:beast-out-chosen-falzar")
local SOUND_GUST = asset.sound("bn6:elec-pulse")

-- Its actions.
local APPEAR, FLASH, RISE, STAY, ATTACK, SINK = 0x0, 0x4, 0x8, 0xC, 0x10, 0x14
-- The actions' ticks.
local APPEAR_TICKS, FLASH_TICKS, RISE_TICKS, STAY_TICKS, SINK_TICKS = 0x3C, 0x0F, 0x1E, 0x78, 0x0A
-- Its height over the user's panel; how long the whirlwind blows; the
-- camera shakes this hard.
local HEIGHT, WHIRLWIND_TICKS, SHAKE = 0x24_0000, 0x78, 3
-- After rising, by the piece's Param1 (0x080C43C4): stay, or attack.
local AFTER_RISE = { [false] = STAY, [true] = ATTACK }

-- The piece's animation `n` (0 to 3).
local function anim(me: Object, n: number): number
    return n + (me.state :: State).anims
end

-- A timed action's first tick: its animation, then `ticks` to count.
local function start(me: Object, n: number, ticks: number)
    me.phase_init = 4
    me.anim = anim(me, n)
    me.timer = ticks
end

-- Count down; at 0 the next action.
local function count(me: Object, next: number)
    local t = me.timer - 1
    me.timer = t
    if t <= 0 then
        me:set_action(next)
    end
end

-- 0x080C4310: shown, with a sound; the leading piece brings the arrival;
-- 60 ticks.
local function appear(me: Object)
    if me.phase_init == 0 then
        me.phase_init = 4
        me.visible = true
        me.timer = APPEAR_TICKS
        battle.play_sound(SOUND_APPEAR)
        if (me.state :: State).leads then
            arrival.spawn(me, SPRITE, ARRIVAL_ANIM, APPEAR_TICKS)
        end
        return
    end
    count(me, FLASH)
end

-- 0x080C4354: white, with a sound; 15 ticks.
local function flash(me: Object)
    if me.phase_init == 0 then
        start(me, 1, FLASH_TICKS)
        battle.play_sound(SOUND_FLASH)
        me.sprite.white = true
        return
    end
    count(me, RISE)
end

-- 0x080C438C: its colours back; 30 ticks; then the other pieces stay and
-- the leading one attacks.
local function rise(me: Object)
    if me.phase_init == 0 then
        start(me, 2, RISE_TICKS)
        me.sprite.white = false
        return
    end
    count(me, AFTER_RISE[(me.state :: State).leads])
end

-- 0x080C43C8: 120 ticks, the first counting down at once.
local function stay(me: Object)
    if me.phase_init == 0 then
        start(me, 3, STAY_TICKS)
    end
    count(me, SINK)
end

-- 0x080C4490: the whirlwind on the panel in front, with its damage word.
local function blow(me: Object)
    local word = bit32.bor(me.damage, bit32.lshift(me.stamina, 16))
    tornado.spawn(me, me.panel_x + me.facing, me.panel_y, me.element, word, WHIRLWIND_TICKS)
end

-- 0x080C43F2: the attack, 120 ticks, the first counting down at once: a
-- cry and a gust, the whirlwind, the camera shaking.
local function attack(me: Object)
    if me.phase_init == 0 then
        start(me, 3, STAY_TICKS)
        battle.play_sound(SOUND_CRY)
        battle.play_sound(SOUND_GUST)
        blow(me)
        battle.shake_camera(SHAKE, me.timer)
    end
    count(me, SINK)
end

-- 0x080C4436: 10 ticks; then the leading piece clears the controller's
-- flag, and it goes.
local function sink(me: Object)
    local s = me.state :: State
    if me.phase_init == 0 then
        start(me, 2, SINK_TICKS)
        return
    end
    local t = me.timer - 1
    me.timer = t
    if t > 0 then
        return
    end
    if s.leads then
        local c = s.controller
        -- (The original writes the byte wherever the controller was.)
        if c and c.active then
            (c.state :: { falzar_out: boolean }).falzar_out = false
        end
    end
    me:set_lifecycle("destroy")
end

-- 0x080C42E6: by action (0x080C42F8).
local function tick(me: Object)
    local a = me.action
    if a == APPEAR then
        appear(me)
    elseif a == FLASH then
        flash(me)
    elseif a == RISE then
        rise(me)
    elseif a == STAY then
        stay(me)
    elseif a == ATTACK then
        attack(me)
    elseif a == SINK then
        sink(me)
    else
        error(string.format("Falzar's summon: action %#x reads past its table (0x080C42F8)", a))
    end
end

-- 0x080C428C: 36 pixels over its panel, the leading piece a pixel up the
-- field and lower; its sprite without a shadow, the piece's first
-- animation, facing its way. (The original decompresses the sprite first:
-- the pack's is ready.)
local function init(me: Object)
    local s = me.state :: State
    me:set_coordinates_from_panel()
    local p = me.pos
    local lead = if s.leads then 0x1_0000 else 0
    me.pos = Vec3.new(p.x, p.y - lead, HEIGHT - lead)
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "with_sprite"
    local a = anim(me, 0)
    me.anim, me.anim_loaded = a, a
    sprite:set_animation(a)
    sprite.palette = 0
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    me:set_lifecycle("update")
end

summon.kind = define.kind {
    id = "bn6:falzar/summon",
    pool = "actor",
    state = { leads = "bool", anims = "u8", controller = "object" },
    -- 0x080C4268: by state (its end `object_freeMemory`), then the sprite,
    -- stepping while dimmed.
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
        me:update_sprite_while_dimmed()
    end,
}

-- 0x080C446A: a piece of Falzar for `user` on (x, y), of its side and
-- facing, with `element` and the damage word, holding `controller`'s flag
-- (set). (Spawned at the registers: the row, the element and `z`; its
-- init places it.)
function summon.spawn(
    controller: Object,
    user: Object,
    x: number,
    y: number,
    element: number,
    damage: number,
    leads: boolean,
    anims: number,
    z: number
): Object?
    local o = battle.spawn(summon.kind, Vec3.new(y, element, z))
    if not o then
        return nil
    end
    o.panel_x, o.panel_y = x, y
    o.element = element
    o.related1 = user
    o.alliance, o.flip = user.alliance, user.flip
    o:set_damage_word(damage)
    local s = o.state :: State
    s.leads, s.anims = leads, anims
    s.controller = controller
    ;(controller.state :: { falzar_out: boolean }).falzar_out = true
    return o
end

return summon
