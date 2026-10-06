--!strict
-- A magnet: MagnetMan's image (EXE5's attack object #0x67, 0x080D2CEC; EXE5's own) on a panel, hitting navis
-- (collision row 0x37, hit modifier 1, the elec spark, immobilizing for 60 ticks), its pole (its first parameter)
-- picking its palette and facing: pole 0 faces its side's way, pole 1 the other. It appears (a sound) and waits
-- for MagnetMan (./navi) to send both (his action 8); then, with its pull (./pull) 20 pixels ahead, it slides 8
-- pixels a tick: off the field, or, when it has a partner (pole 1 always), to 10 pixels short of the panel of
-- MagnetMan's target, where the two meet. A navi it hits, or the meeting, stops it: it shakes (a sound) 11
-- ticks, waits 6 and goes. Its variant 1 hits with modifier 3 and without the immobilization, goes at once and
-- stops while dimmed: MagnetMan the navi's own chip's (navis/magnetman/chip), and EXE5's MagnetMan as an enemy's.

local collision = require("../../rules/collision")
local navi_chips = require("@exelib/navi_chips/navi")
local pull = require("./pull")
local regions = require("@exelib/regions")
local sparks = require("../../lib/sparks")
local statuses = require("../../rules/status")

export type State = {
    -- Its pole (its first parameter: 0, 1), it has a partner (its second), the column of MagnetMan's target (its
    -- third), and its variant (its fourth: 0 for the chip).
    pole: number,
    paired: number,
    target_x: number,
    variant: number,
    -- The chip's level (+0x60), which picks its palette.
    level: number,
    -- It hit a navi (+0x64).
    hit_navi: boolean,
    -- Where it stops (+0x48: 10 pixels short of the target panel's center).
    stop_x: number,
    -- Its shake (+0x44: 2 pixels one way, then the other).
    shake: number,
}

local magnet = {}

local SPRITE = asset.sprite("magnetman")
local SOUND_APPEAR = asset.sound("appear")
local SOUND_CRASH = asset.sound("magnet-crash")
-- By the level (0x0800DA8A: navi 5's count, 2, by 0x0800DAE8's row), then its pole added: DS's 6.
local PALETTES: { [number]: number } = { [0] = 0, 0, 0, 4, 6 }
-- By the variant (0x080D2DE0, 0x080D2DEC): the hit modifier and the status.
local HIT_MODS: { [number]: number } = { [0] = 1, 3 }
local STATUSES: { [number]: Status? } = { [0] = statuses.immobilize_60 }
-- A navi's body among what it hit.
local NAVI_HIT = 0x0C00_0000
local SPEED = 0x8_0000

local function variant_of(me: Object): number
    local v = (me.state :: State).variant
    if v > 1 then
        error(string.format("a magnet's variant %d reads past 0x080D2DE0's table", v))
    end
    return v
end

-- Its facing, as its flip reads it: its pole, its side and its flip together.
local function turned(me: Object): number
    return bit32.bxor((me.state :: State).pole, me.alliance, me.flip)
end

local function stop(me: Object)
    me.collision:set_region(nil)
    me:set_action(8)
end

-- Action 0 (0x080D2E4C; its phase byte the entry flag): where it stops; then it waits for MagnetMan to be done
-- sending (or not, variant 1), and goes with its pull ahead, its panel hit.
local function wait(me: Object)
    local s = me.state :: State
    if me.phase == 0 then
        me.phase = 4
        local x = field.center(s.target_x, me.panel_y)
        local dir = if turned(me) ~= 0 then -1 else 1
        s.stop_x = int.i32(x - dir * 0xA_0000)
        return
    end
    if s.variant == 1 or (me.related1 :: Object).action == 8 then
        me.anim = 0x11
        local dir = if me.vel.x > 0 then 1 else -1
        pull.spawn(me, me.panel_x + dir, me.panel_y, me.element, dir)
        me.collision:set_region(regions.single)
        me:set_action(4)
    end
end

-- Action 4 (0x080D2EBE): it slides, its region following it panel to panel; a navi hit or the meeting point (with
-- a target column, for pole 1 or a pair) stops it, and so does the field's end.
local function slide(me: Object)
    local s = me.state :: State
    local c = me.collision
    if s.hit_navi then
        stop(me)
        return
    end
    local flags = c.hit_flags
    if flags ~= 0 then
        if bit32.band(flags, NAVI_HIT) ~= 0 then
            stop(me)
            return
        end
        c:set_region(nil)
    end
    local from = me.panel_x
    local p = me.pos
    local x = int.i32(p.x + me.vel.x)
    me.pos = Vec3.new(x, p.y, p.z)
    if s.target_x ~= 0 and (s.pole ~= 0 or s.paired ~= 0) then
        if navi_chips.between(x, p.x, s.stop_x) then
            stop(me)
            return
        end
        me:set_panel_from_coordinates()
        me:update_collision_panels()
        if me.panel_x ~= from then
            c:set_region(regions.single)
            return
        end
    end
    me:set_panel_from_coordinates()
    me:update_collision_panels()
    if me.panel_x ~= from then
        c:set_region(regions.single)
    end
    if not field.valid(me.panel_x, me.panel_y) then
        stop(me)
    end
end

-- Action 8 (0x080D2F48; its phase byte the entry flag): the crash (a sound): 11 ticks shaking 2 pixels every third.
local function crash(me: Object)
    local s = me.state :: State
    if me.phase == 0 then
        me.phase = 4
        battle.play_sound(SOUND_CRASH)
        s.shake = if me.vel.x < 0 then 0x2_0000 else -0x2_0000
        me.timer = 0xA
        return
    end
    if me.timer % 3 == 0 then
        local p = me.pos
        me.pos = Vec3.new(int.i32(p.x + s.shake), p.y, p.z)
        s.shake = -s.shake
    end
    local left = me.timer - 1
    me.timer = int.u16(left)
    if left < 0 then
        me:set_action(0xC)
    end
end

-- Action 0xC (0x080D2F96; its phase byte the entry flag): 6 ticks; then it goes.
local function linger(me: Object)
    if me.phase == 0 then
        me.phase = 4
        me.timer = 5
        return
    end
    local left = me.timer - 1
    me.timer = int.u16(left)
    if left <= 0 then
        -- (0x080D2FB2: the state byte alone.)
        me.lifecycle = "destroy"
    end
end

local ACTIONS = { [0] = wait, [4] = slide, [8] = crash, [0xC] = linger }

-- 0x080D2DF4: the hits against it resolve and spark (a navi's body among them stops it); its action (variant 1's
-- not while dimmed).
local function tick(me: Object)
    local c = me.collision
    c:remove()
    me:hit_spark()
    local flags = c.hit_flags
    if flags ~= 0 then
        c:set_region(nil)
        if bit32.band(flags, NAVI_HIT) ~= 0 then
            (me.state :: State).hit_navi = true
        end
    end
    if variant_of(me) == 0 or not battle.dimmed() then
        local action = ACTIONS[me.action]
        if not action then
            error(string.format("A magnet's action %#x reads past its table (0x080D2E38)", me.action))
        end
        action(me)
    end
    c:present()
end

-- 0x080D2D20: on its panel, a pixel up and nearer; MagnetMan's sprite (animation 17, his shadow), shown, its
-- palette by the level and its pole, facing by its pole; its speed that way; its collision; a sound.
local function init(me: Object)
    local s = me.state :: State
    local variant = variant_of(me)
    me:set_coordinates_from_panel()
    me.future_panel_x = me.panel_x
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "ground"
    me.visible = true
    local p = me.pos
    me.pos = Vec3.new(p.x, int.i32(p.y + 0x1_0000), -0x1_0000)
    me.anim, me.anim_loaded = 0x11, 0x11
    sprite:set_animation(0x11)
    sprite.palette = (PALETTES[s.level] or 0) + s.pole
    local turn = turned(me)
    sprite:set_flip(turn)
    me.vel = Vec3.new(if turn ~= 0 then -SPEED else SPEED, me.vel.y, me.vel.z)
    if not me:create_collision() then
        me:free()
        return
    end
    me:setup_collision(collision.collision_37, collision.hits_navis, HIT_MODS[variant])
    local c = me.collision
    c:set_hit_effect(sparks.elec)
    c.status_base = STATUSES[variant]
    c:set_region(nil)
    c:present()
    battle.play_sound(SOUND_APPEAR)
    s.hit_navi = false
    me:set_lifecycle("update")
end

magnet.kind = define.kind {
    id = "magnetmn/magnet",
    pool = "attack",
    state = {
        pole = "u8",
        paired = "u8",
        target_x = "u8",
        variant = "u8",
        level = "u8",
        hit_navi = "bool",
        stop_x = "i32",
        shake = "i32",
    },
    -- 0x080D2CEC.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
            if not me.active then
                return
            end
        elseif l == "update" then
            tick(me)
        else
            me:destroy()
            return
        end
        if (me.state :: State).variant ~= 0 then
            if not battle.dimmed() then
                me:update_sprite()
            end
        else
            me:update_sprite_while_dimmed()
        end
    end,
}

export type Spec = {
    pole: number,
    paired: boolean,
    target_x: number,
    -- Its variant (none: 0, the chips').
    variant: number?,
    -- The chip's level, and the damage word.
    level: number,
    damage: number,
}

-- 0x080D2FC0: a magnet of `navi`'s on (x, y) with `element`, called while the battle is dimmed too. (Its position
-- is the spawner's registers, which its init overwrites.)
function magnet.spawn(navi: Object, x: number, y: number, element: number, spec: Spec): Object?
    local o = battle.spawn(magnet.kind, Vec3.zero)
    if o then
        local s = o.state :: State
        s.pole, s.paired, s.target_x = spec.pole, if spec.paired then 1 else 0, spec.target_x
        s.variant = spec.variant or 0
        s.level = spec.level
        o.panel_x, o.panel_y = x, y
        o.element = element
        o.related1 = navi
        o:set_damage_word(spec.damage)
        o.alliance, o.flip = navi.alliance, navi.flip
        o.run_while_dimmed = true
    end
    return o
end

return magnet
