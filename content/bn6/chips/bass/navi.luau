--!strict
-- Bass, the navi of the Bass chip (the original's actor object #0x4F,
-- `sub_80C3970`), brought by its navi chip controller. He appears on his
-- user's panel with his cape (a form overlay), rises for 100 ticks, then
-- fires 24 volleys of two shots (objects/panel-strike), 8 ticks apart, at
-- the columns ahead
-- holding the other side's panels: each shot at a panel with an enemy navi
-- five times in eight (else, or if there is none, any panel of those
-- columns), none at a panel shot in the last 22 ticks. Then he leaves, and
-- the controller moves on. See docs/engine/chips.md §3.6.33.

local panels = require("../../lib/panels")
local panel_strike = require("../../objects/panel-strike/panel_strike")
local effects = require("../../lib/effects")

export type State = {
    -- The controller waiting for him (the game keeps its pointer in his X
    -- velocity).
    controller: Object?,
    -- Ticks before each panel can be shot again, by (y - 1) * 6 + x
    -- (object +0x60).
    cooldowns: { number },
    -- The columns he shoots at, nearest first, 0-terminated (object +0x74).
    columns: { number },
}

local bass = {}

local SPRITE = asset.sprite("bn6:bass-anly")
local SOUND_APPEAR = asset.sound("bn6:appear")

-- His cape: a form overlay on his animation + 0x14, stepping while
-- dimmed.
local CAPE_ANIM_OFFSET = 0x14

-- The columns he shoots at, by side (`byte_80C3C90`: the other side's
-- panels), and the panels his shots prefer (`off_80C3BB0`: an enemy navi's
-- body), as {require, forbid}; otherwise any panel with 0x10000.
local COLUMN_FILTER = { [0] = { 0x20, 0 }, { 0, 0x20 } }
local ENEMY_NAVI = { [0] = { 0x0400_0000, 0 }, { 0x0800_0000, 0 } }
local ANY_PANEL = 0x1_0000

-- His shots: 20 ticks of warning, hit modifier 3, on solid panels only.
local SHOT: PanelStrike = { warning = 0x14, hit_mod = 3, solid_only = true }
local SHOT_COOLDOWN = 0x16
-- A shot's flash at his hand (effect 0x57).
local FLASH = define.effect { sprite = asset.sprite("bn6:hit-sparks"), anim = 8, palette = 0 }

local function damage_word(me: Object): number
    return bit32.bor(me.damage, bit32.lshift(me.stamina, 16))
end

-- `sub_80C3C2C`: the columns ahead, from the third (or his own, if
-- further), that hold a panel of the other side's; the first, or 0.
local function find_columns(me: Object): number
    local s = me.state :: State
    local x = me.panel_x
    local facing_right = bit32.bxor(me.alliance, me.flip) == 0
    if facing_right then
        if x < 3 then
            x = 3
        end
    elseif x > 4 then
        x = 4
    end
    local last = if facing_right then 6 else 1
    local filter = COLUMN_FILTER[me.alliance]
    local n = 0
    while x ~= last do
        x += me.facing
        if #panels.in_column(x, filter[1], filter[2]) ~= 0 then
            n += 1
            s.columns[n] = x
        end
    end
    s.columns[n + 1] = 0
    return s.columns[1]
end

-- `sub_80C3BC0`: a random panel of his columns, rows 3 to 1, whose flags
-- have every `require` bit and no `forbid` bit and that isn't cooling
-- down (one RNG draw when there is any).
local function pick(me: Object, require: number, forbid: number): (number?, number?)
    local s = me.state :: State
    local found = {}
    local i = 1
    while s.columns[i] ~= 0 do
        local x = s.columns[i]
        for y = 3, 1, -1 do
            local f = field.flags(x, y)
            if bit32.band(f, forbid) == 0 and bit32.band(f, require) == require and s.cooldowns[(y - 1) * 6 + x] == 0 then
                table.insert(found, bit32.bor(bit32.lshift(y, 4), x))
            end
        end
        i += 1
    end
    if #found == 0 then
        return nil, nil
    end
    local p = found[battle.rng_positive() % #found + 1]
    return panels.x(p), panels.y(p)
end

-- `sub_80C3B54`: a shot, and its flash at his hand.
local function shoot(me: Object)
    if find_columns(me) == 0 then
        return
    end
    local px: number?, py: number? = nil, nil
    if bit32.band(battle.rng_positive(), 7) >= 3 then
        local f = ENEMY_NAVI[me.alliance]
        px, py = pick(me, f[1], f[2])
    end
    if not px then
        px, py = pick(me, ANY_PANEL, 0)
    end
    if not px or not py then
        return
    end
    local x, y = px :: number, py :: number
    local s = me.state :: State
    s.cooldowns[(y - 1) * 6 + x] = SHOT_COOLDOWN
    panel_strike.spawn(me, x, y, 0, SHOT, damage_word(me), 0, true)
    -- sub_80C3CA0: the flash 20 pixels ahead of him and 34 up, jittered.
    local p = me.pos
    local pos = battle.jitter(0xF, Vec3.new(p.x + me.facing * 0x14_0000, p.y, p.z + 0x22_0000))
    battle.effect(pos, FLASH, bit32.bxor(me.alliance, me.flip))
end

-- `sub_80C3CD0`: every panel's cooldown counts down.
local function cool_down(me: Object)
    local c = (me.state :: State).cooldowns
    for i = 18, 1, -1 do
        if c[i] >= 1 then
            c[i] -= 1
        end
    end
end

-- `sub_80C3A50`: 100 ticks, rising half a pixel a tick over the last 81.
local function rise(me: Object)
    if me.timer <= 0x50 then
        local p = me.pos
        me.pos = Vec3.new(p.x, p.y, p.z + 0x8000)
    end
    local left = me.timer - 1
    me.timer = left
    if left < 0 then
        me:set_action(4)
    end
end

-- `sub_80C3A98`: he raises his arm (animation 0xA); when it ends, the
-- volleys.
local function raise(me: Object)
    if me.phase_init == 0 then
        me.phase_init = 1
        me.anim = 0xA
    end
    if bit32.band(me.sprite.frame_flags, 0x80) == 0 then
        return
    end
    me.anim = 0xC
    me.timer2 = 0x18
    local c = (me.state :: State).cooldowns
    for i = 1, 18 do
        c[i] = 0
    end
    me.phase, me.phase_init = 4, 0
end

-- `sub_80C3ACA`: a volley of two shots every 8 ticks, 24 of them; then 12
-- ticks.
local function volleys(me: Object)
    cool_down(me)
    if me.phase_init == 0 then
        me.phase_init = 1
        shoot(me)
        shoot(me)
        local left = me.timer2 - 1
        me.timer2 = left
        if left == 0 then
            me.timer = 0xC
            me.phase, me.phase_init = 8, 0
            return
        end
        me.timer = 8
    end
    local left = me.timer - 1
    me.timer = left
    if left == 0 then
        me.phase_init = 0
    end
end

-- `sub_80C3B06`: the 12 ticks, then his arm down and 30 more; then he
-- goes.
local function lower(me: Object)
    if me.phase_init == 0 then
        local left = me.timer - 1
        me.timer = left
        if left ~= 0 then
            return
        end
        me.phase_init = 1
        me.timer = 0x1E
        me.anim = 0
    end
    local left = me.timer - 1
    me.timer = left
    if left == 0 then
        me:set_lifecycle("destroy")
    end
end

local PHASES = { [0] = raise, [4] = volleys, [8] = lower }

-- `sub_80C3A78`.
local function attack(me: Object)
    local f = PHASES[me.phase]
    if not f then
        error(string.format("Bass's attack phase %#x reads past its table (off_80C3A8C)", me.phase))
    end
    f(me)
end

local ACTIONS = { [0] = rise, [4] = attack }

-- `sub_80C3A24`: his action; gone if his panel is off the field.
local function tick(me: Object)
    local action = ACTIONS[me.action]
    if not action then
        error(string.format("Bass action %#x reads past his table (off_80C3A48)", me.action))
    end
    action(me)
    if not field.valid(me.panel_x, me.panel_y) then
        me:set_lifecycle("destroy")
    end
end

-- `sub_80C39BA`: on his panel, on the ground, with his cape; then his
-- first tick.
local function init(me: Object)
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "ground"
    me.visible = true
    me.anim = 0
    me.anim_loaded = 0
    sprite:set_animation(0)
    sprite.palette = 0
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    me:set_coordinates_from_panel()
    local p = me.pos
    me.pos = Vec3.new(p.x, p.y, 0)
    me.related1 = battle.form_overlay(me, { sprite = SPRITE, stepping = "while_dimmed", anim_offset = CAPE_ANIM_OFFSET })
    battle.play_sound(SOUND_APPEAR)
    me.timer = 0x64
    me:set_lifecycle("update")
    tick(me)
end

-- `sub_80C3994`: a burst where he stood, his cape off, and the controller
-- moves on.
local function leave(me: Object)
    local p = me.pos
    battle.effect(Vec3.new(p.x, p.y, p.z + 0x10_0000), effects.smoke)
    local cape = me.related1
    if cape then
        cape:set_lifecycle("destroy")
    end
    local controller = (me.state :: State).controller
    if controller then
        navi_chip.navi_left(controller)
    end
    me:free()
end

bass.kind = define.kind {
    id = "bn6:bass/navi",
    pool = "actor",
    state = { controller = "object", cooldowns = "u8[18]", columns = "u8[7]" },
    -- `sub_80C3970`.
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

-- `sub_80C3B30`: Bass for `user`, reporting to `controller` (the Bass
-- chip's navi hook, `off_802CD5C[26]`). (His position is the spawner's
-- registers, which his init overwrites.)
function bass.summon(user: Object, controller: Object, spec: NaviChipSpec): Object?
    local o = battle.spawn(bass.kind, Vec3.zero)
    if not o then
        return nil
    end
    o.panel_x, o.panel_y = spec.panel_x, spec.panel_y
    o.element = spec.element
    o.alliance, o.flip = user.alliance, user.flip
    o:set_damage_word(spec.damage)
    local s = o.state :: State
    s.controller = controller
    return o
end

return bass
