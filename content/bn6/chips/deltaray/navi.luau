--!strict
-- DeltaRay, the navi of the DeltaRay chip (the original's actor object
-- #0x3D, `sub_80C2A78`), brought by its navi chip controller. He fades in
-- on his user's panel, flashing, then looks for the first enemy navi ahead
-- (column by column, rows 1 to 3) and a free panel diagonally behind and
-- above it; there he strikes with his sword (a column of three in front of
-- him, with a slash effect). Each time his user presses A while dimmed
-- during a strike, he goes on to the next corner of the delta (up to three
-- strikes), as long as the panel is free; then he returns home, flashes
-- the screen with a burst, and leaves. Without an enemy navi ahead, he
-- only marks the air. See docs/engine/chip-table.md (navi chip
-- subtype 28).

local attachment = require("../../objects/attachment/attachment")
local follow_effect = require("../../objects/follow-effect/follow_effect")
local parts = require("../../lib/swords/parts")
local collision = require("../../rules/collision")
local effects = require("../../lib/effects")
local regions = require("../../lib/regions")

-- What a chip summons him with: his palette (the chip's first parameter
-- picks it).
export type Summon = { palette: number }

export type State = {
    -- His palette (from Param1).
    palette: number,
    -- The controller waiting for him (its pointer sits in his
    -- ExtraVars[1]).
    controller: Object?,
    -- His facing for the strike (ExtraVars +0x0C; the sprite's flip value).
    facing: number,
    -- His home panel (y << 8 | x, ExtraVars +0x10).
    home: number,
    -- The target panel found (+0x14, +0x15), and the first candidate
    -- (+0x16, +0x17) with its flag (+0x18: its x).
    target_x: number,
    target_y: number,
    first_x: number,
    first_y: number,
    first_found: number,
    -- The user asked for another strike (+0x19).
    again: number,
    -- How white he is, 0 to 31 (his header's byte +0x19).
    glow: number,
}

local deltaray = {}

local SPRITE = asset.sprite("protoman")
local SOUND_APPEAR = asset.sound("appear")
local SOUND_COPY_MARK = asset.sound("mark")
local SOUND_DELTA_RAY_2 = asset.sound("delta-ray-2")
local SOUND_LOG_IN_77 = asset.sound("log-in")

-- The flash above him as he arrives (effect 0x03), and the mark on his
-- target (effect 0x28).
local FLASH = define.effect { sprite = asset.sprite("deletion"), anim = 0, palette = 0 }
local MARK = define.effect { sprite = asset.sprite("hit-sparks"), anim = 11, palette = 1 }

-- What he looks for (by side, `off_80C2F2C`: an enemy navi's body), and a
-- panel he can stand on: solid, and none of these (by side).
local ENEMY_NAVI = { [0] = 0x0400_0000, 0x0800_0000 }
local STAND_FORBID = { [0] = 0x0788_0080, 0x0B88_0080 }

-- The delta's steps from one corner to the next, x toward his front
-- (`byte_80C2F20`), by strike.
local STEPS = { [0] = { -1, -1 }, { 2, 1 }, { -2, 1 }, { 0, 0 } }

-- His sword: the swords' blade (the original's attachment 3), its
-- animation 0x0B, stepping while dimmed; his slash drawn over the column
-- (effect 0x27).
local SWORD = parts.blades.sword
local SWORD_ANIM = 0x0B
local SLASH = parts.effects.wide

-- His bursts' look (the follow effect's look 4).
local BURST = follow_effect.look { sprite = asset.sprite("follow-effect"), anim = 0, palette = 0, behavior = "flash" }

local function st(me: Object): State
    return me.state :: State
end

-- Enter the phase with `ticks`, or count down; true once 0 or below.
local function countdown(me: Object, ticks: number): boolean
    if me.phase_init == 0 then
        me.phase_init = 4
        me.timer = ticks
        return false
    end
    local left = me.timer - 1
    me.timer = left
    return left <= 0
end

local function next_phase(me: Object, phase: number)
    me.phase, me.phase_init = phase, 0
end

-- `sub_80C2B3C`: he fades in (flickering), losing his glow 2 a tick.
local function fade_in(me: Object)
    local s = st(me)
    if me.phase_init == 0 then
        s.glow = 0x1F
        battle.play_sound(SOUND_APPEAR)
        me.timer = 0
        me.phase_init = 4
        return
    end
    me.visible = true
    local t = me.timer + 1
    me.timer = t
    if bit32.band(t, 1) ~= 0 then
        me.visible = false
    end
    local glow = s.glow - 2
    s.glow = glow
    if glow > 0 then
        return
    end
    me.visible = true
    s.glow = 0
    next_phase(me, 4)
end

-- `sub_80C2B8C`: a flash above him, 60 ticks; then
-- on to the attack, or gone if his panel isn't solid.
local function arrive(me: Object)
    if me.phase_init == 0 then
        me.timer = 0x3C
        local p = me.pos
        battle.effect(Vec3.new(p.x, p.y, p.z + 0x14_0000), FLASH)
        battle.play_sound(SOUND_LOG_IN_77)
        me.phase_init = 4
        return
    end
    local left = me.timer - 1
    me.timer = left
    if left > 0 then
        return
    end
    if not field.check(me.panel_x, me.panel_y, 0x10, 0) then
        me:set_lifecycle("destroy")
        return
    end
    st(me).facing = bit32.bxor(me.alliance, me.flip)
    me:set_action(4)
end

local ARRIVE_PHASES = { [0] = fade_in, [4] = arrive }

-- `sub_80C2B20`.
local function arriving(me: Object)
    local f = ARRIVE_PHASES[me.phase]
    if not f then
        error(string.format("DeltaRay phase %#x reads past his table (off_80C2B34)", me.phase))
    end
    f(me)
end

-- A panel he can stand on.
local function can_stand(me: Object, x: number, y: number): boolean
    return field.check(x, y, 0x10, STAND_FORBID[me.alliance])
end

-- `sub_80C2C14`: from the column ahead, the first enemy navi (rows 1 to
-- 3) whose panel behind and above is one he can stand on: its panel is
-- his target (FuturePanel, the delta's centre), 30 ticks. None: 15 ticks,
-- and Timer2 0xFF.
local function search(me: Object)
    local s = st(me)
    s.first_found, s.first_x, s.first_y = 0, 0, 0
    local f = me.facing
    local x = me.panel_x + f
    while true do
        for y = 1, 3 do
            if field.check(x, y, ENEMY_NAVI[me.alliance], 0) then
                local cx, cy = x - f, y - 1
                if s.first_found == 0 then
                    s.first_found = cx
                    s.first_x, s.first_y = cx, cy
                end
                if field.valid(cx, cy) then
                    s.target_x, s.target_y = cx, cy
                    if can_stand(me, cx, cy) then
                        me.future_panel_x, me.future_panel_y = x, y
                        me.timer2 = 0
                        me.timer = 0x1E
                        next_phase(me, 4)
                        return
                    end
                end
            end
        end
        x += f
        if not field.valid(x, 1) then
            break
        end
    end
    me.timer2 = 0xFF
    me.timer = 0xF
    next_phase(me, 4)
end

-- `sub_80C2CB4`: a mark (lasting his timer) on the target panel; then the
-- strike, or (no target) the end.
local function mark(me: Object)
    if me.phase_init == 0 then
        local cx, cy = field.center(me.future_panel_x, me.future_panel_y)
        local fx = battle.effect(Vec3.new(cx, cy, 0), MARK)
        if fx then
            fx.timer = me.timer
        end
        battle.play_sound(SOUND_COPY_MARK)
        me.phase_init = 4
        return
    end
    local left = me.timer - 1
    me.timer = left
    if left > 0 then
        return
    end
    if me.timer2 ~= 0 then
        me:set_action(8)
        return
    end
    st(me).again = 1
    next_phase(me, 8)
end

-- `sub_80C2EC4`: step `n` of the delta: his facing for it, the next corner
-- (FuturePanel), Timer2 n + 1; whether he can stand there.
local function step(me: Object, n: number): boolean
    st(me).facing = bit32.bxor(bit32.band(n, 1), bit32.bxor(me.alliance, me.flip))
    local d = STEPS[n]
    if not d then
        error(string.format("DeltaRay step %d reads past byte_80C2F20", n))
    end
    me.future_panel_x += d[1] * me.facing
    me.future_panel_y += d[2]
    me.timer2 = n + 1
    local x, y = me.future_panel_x, me.future_panel_y
    return field.valid(x, y) and can_stand(me, x, y)
end

-- `loc_80C2D06`: vanishing (animation 4, 3 ticks, his sword let go); then
-- home after the third strike, the next strike if asked for and possible,
-- or the end.
local function vanish(me: Object)
    if me.phase_init == 0 then
        me.related1 = nil
        me.anim = 4
        me.timer = 3
        me.phase_init = 4
        return
    end
    local left = me.timer - 1
    me.timer = left
    if left > 0 then
        return
    end
    if me.timer2 == 3 then
        return next_phase(me, 0x14)
    end
    local s = st(me)
    if s.again ~= 0 and step(me, me.timer2) then
        s.again = 0
        return next_phase(me, 0xC)
    end
    me:set_action(8)
end

-- `sub_80C2D56`: appearing on the corner (animation 3), 3 ticks.
local function appear_at(me: Object)
    if me.phase_init == 0 then
        me.anim = 3
        me.sprite:set_flip(st(me).facing)
        me.timer = 3
        me.panel_x, me.panel_y = me.future_panel_x, me.future_panel_y
        me:set_coordinates_from_panel()
        me.phase_init = 4
        return
    end
    local left = me.timer - 1
    me.timer = left
    if left <= 0 then
        next_phase(me, 0x10)
    end
end

-- `sub_80C2EB0`: his side's player pressed A while dimmed.
local function asked_again(me: Object): boolean
    local p = battle.player(me.alliance) :: Object
    return p:dimmed_pressed("a")
end

-- `sub_80C2D8C`: the strike: his sword out (animation 5), 20 ticks; at 10
-- left a flash, the hit (a column of three, 16 pixels up) and the slash on
-- the panel he faces. A press while more than 5 are left asks
-- for another.
local function strike(me: Object)
    local s = st(me)
    if me.phase_init == 0 then
        me.anim = 5
        me.anim_loaded = 6
        attachment.attach(me, SWORD, "related", { anim = SWORD_ANIM, while_dimmed = true })
        battle.play_sound(parts.sounds.swing)
        me.timer = 0x14
        me.phase_init = 4
        return
    end
    local t = me.timer
    if t > 5 and asked_again(me) then
        s.again = 1
    end
    if t == 0xA then
        battle.palette_flash(0, 5, true)
        local x, y = me.panel_x + (1 - 2 * s.facing), me.panel_y
        local box = battle.hitbox(me, {
            panel_x = x,
            panel_y = y,
            element = 0,
            z = 0x10_0000,
            region = regions.wide,
            hit_effect = nil,
            target = collision.hits_navis,
            self_type = collision.slash,
            damage = me.damage,
            stamina = me.stamina,
            hit_mod = 3,
        })
        if box then
            box.run_while_dimmed = true
        end
        local cx, cy = field.center(x, y)
        battle.effect(Vec3.new(cx, cy, 0x10_0000), SLASH, s.facing)
    end
    local left = t - 1
    me.timer = left
    if left <= 0 then
        next_phase(me, 8)
    end
end

-- `sub_80C2E30`: back home (animation 3, facing away), 3 ticks.
local function home(me: Object)
    if me.phase_init == 0 then
        me.related1 = nil
        me.anim = 3
        me.sprite:set_flip(bit32.bxor(bit32.bxor(me.alliance, me.flip), 1))
        me.timer = 3
        local h = st(me).home
        me.panel_x, me.panel_y = bit32.band(h, 0xFF), bit32.rshift(h, 8)
        me:set_coordinates_from_panel()
        me.phase_init = 4
        return
    end
    local left = me.timer - 1
    me.timer = left
    if left <= 0 then
        next_phase(me, 0x18)
    end
end

-- `sub_80C2FBC`: a burst (a follow effect's look 4, 60 ticks) on (x, y),
-- and a white flash of 20 ticks.
local function burst(me: Object, x: number, y: number)
    local cx, cy = field.center(x, y)
    local fx = follow_effect.spawn(me, Vec3.new(cx, cy, 0), BURST, bit32.bxor(me.alliance, me.flip))
    if fx then
        fx.timer = 0x3C
    end
    battle.play_sound(SOUND_DELTA_RAY_2)
    battle.palette_flash(0, 0x14, true)
end

-- `sub_80C2E6E`: the finish (animation 0xF), a burst in the middle row
-- beside his last corner, 60 ticks (to 0); then the end.
local function finish(me: Object)
    if me.phase_init == 0 then
        me.related1 = nil
        me.anim = 0xF
        me.timer = 0x3C
        local x = me.future_panel_x + (if bit32.bxor(me.alliance, me.flip) ~= 0 then -1 else 1)
        burst(me, x, 2)
        me.phase_init = 4
        return
    end
    local left = me.timer - 1
    me.timer = left
    if left == 0 then
        me:set_action(8)
    end
end

local ATTACK_PHASES = {
    [0] = search,
    [4] = mark,
    [8] = vanish,
    [0xC] = appear_at,
    [0x10] = strike,
    [0x14] = home,
    [0x18] = finish,
}

-- `sub_80C2BE6`.
local function attacking(me: Object)
    local f = ATTACK_PHASES[me.phase]
    if not f then
        error(string.format("DeltaRay phase %#x reads past his table (off_80C2BF8)", me.phase))
    end
    f(me)
end

-- `sub_80C2F40`: vanishing (animation 4, 5 ticks), then gone.
local function leave(me: Object)
    if me.phase == 0 then
        if me.anim ~= 4 then
            me.anim = 4
            me.related1 = nil
            me.timer = 5
            me.phase = 4
            return
        end
    else
        local left = me.timer - 1
        me.timer = left
        if left > 0 then
            return
        end
    end
    me.visible = false
    me:set_lifecycle("destroy")
end

local ACTIONS = { [0] = arriving, [4] = attacking, [8] = leave }

-- `sub_80C2AF8`: act; step the sprite (while dimmed too); his glow tints
-- him white (`sub_80C2F84`).
local function tick(me: Object)
    local action = ACTIONS[me.action]
    if not action then
        error(string.format("DeltaRay action %#x reads past his table (off_80C2B10)", me.action))
    end
    action(me)
    me:update_sprite_while_dimmed()
    me.sprite.color_shader = st(me).glow * 0x421
end

-- `sub_80C2A9C`: on his panel, on the ground, white; then his first
-- update.
local function init(me: Object)
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "ground"
    me.anim = 0
    me.anim_loaded = 0
    sprite:set_animation(0)
    me:set_coordinates_from_panel()
    local p = me.pos
    me.pos = Vec3.new(p.x, p.y, 0)
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    st(me).home = bit32.bor(me.panel_x, bit32.lshift(me.panel_y, 8))
    sprite.palette = st(me).palette
    sprite.color_shader = 0x7FFF
    me.visible = true
    me:set_lifecycle("update")
    tick(me)
end

-- `sub_80C2F76`: the controller moves on, and he goes.
local function gone(me: Object)
    local controller = st(me).controller
    if controller then
        navi_chip.navi_left(controller)
    end
    me:free()
end

deltaray.kind = define.kind {
    id = "deltaray/navi",
    pool = "actor",
    state = {
        controller = "object",
        facing = "u8",
        home = "u16",
        target_x = "u8",
        target_y = "u8",
        first_x = "u8",
        first_y = "u8",
        first_found = "u8",
        again = "u8",
        glow = "u8",
        palette = "u8",
    },
    -- `sub_80C2A78`.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
        elseif l == "update" then
            tick(me)
        else
            gone(me)
        end
    end,
}

-- `sub_80C2F96`: the DeltaRay chip's navi hook (`off_802CD5C[28]`):
-- DeltaRay for `user`, reporting to `controller`. (His position is the
-- spawner's registers, all of which his init overwrites.)
function deltaray.summon(args: Summon): (user: Object, controller: Object, spec: NaviChipSpec) -> Object?
    return function(user: Object, controller: Object, spec: NaviChipSpec): Object?
        local o = battle.spawn(deltaray.kind, Vec3.zero)
        if not o then
            return nil
        end
        o.panel_x, o.panel_y = spec.panel_x, spec.panel_y
        o.element = spec.element
        o.alliance, o.flip = user.alliance, user.flip
        o.related1 = user
        o:set_damage_word(spec.damage)
        local s = o.state :: State
        s.controller = controller
        s.palette = args.palette
        return o
    end
end

return deltaray
