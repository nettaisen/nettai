--!strict
-- K-BOMB's bombs (the DS's attack object 0xCC, 0x02015F4C): a bomb (0C-02) lobbed at a corner of the panel three
-- columns ahead of its thrower's, falling 60 ticks (halfway it turns into NumberMan's press, 10-25). It lands (on the
-- ground over a solid panel of its thrower's, else in a puff of smoke, hidden), then bursts: four hits on its quarter
-- of the 3x3 around the target panel (the target and three panels toward its corner; no spark, hitting navis, hit
-- modifier 3, confusing for 480 ticks), the camera shaking, a sound; 20 ticks on it goes in a puff.

local collision = require("../../rules/collision")
local effects = require("../../lib/effects")
local regions = require("@exelib/regions")
local statuses = require("../../rules/status")
local ds_arc = require("../../lib/ds_arc")

local bomb = {}

type State = {
    -- Its quarter (the DS's Param1): 0 behind and above the target, 1 ahead and above, 2 ahead and below, 3 behind and
    -- below.
    quarter: number,
}

local SPRITE = asset.sprite("bomb")
local PRESS = asset.sprite("numtrap-press")
local SOUND_BURST = asset.sound("drumroll")
-- The target: three columns ahead. Its flight (60 ticks, falling 0.75 a tick), its turn into the press halfway.
local AHEAD = 3
local FLIGHT, TURNS_AT = 0x3C, 0x1E
local GRAVITY = -0xC000
-- Where it lands in pixels from the target's center, by quarter (0x020D80C0: x toward its front).
local CORNERS = { { -20, -12 }, { 20, -12 }, { 20, 12 }, { -20, 12 } }
-- Its hits' panels from the target, by quarter (0x020D80DC: x toward its front).
local HITS = {
    { { 0, 0 }, { -1, 0 }, { -1, -1 }, { 0, -1 } },
    { { 0, 0 }, { 0, -1 }, { 1, -1 }, { 1, 0 } },
    { { 0, 0 }, { 1, 0 }, { 1, 1 }, { 0, 1 } },
    { { 0, 0 }, { 0, 1 }, { -1, 1 }, { -1, 0 } },
}
local BURST_TICKS = 0x14
local SMOKE_Z, PUFF_Z = 0x14_0000, 0xA_0000
local FLY, BURST = 0x0, 0x4

-- Action 0 (0x02015FFC): its flight, 60 ticks; halfway the press; then it lands.
local function fly(me: Object)
    if me.phase_init == 0 then
        me.timer = FLIGHT
        me.phase_init = 4
    end
    local p, v = me.pos, me.vel
    local vz = int.i32(v.z + GRAVITY)
    me.vel = Vec3.new(v.x, v.y, vz)
    me.pos = Vec3.new(int.i32(p.x + v.x), int.i32(p.y + v.y), int.i32(p.z + vz))
    if me.timer == TURNS_AT then
        me.sprite:load(PRESS)
        me.sprite.shadow = "ground"
        me.anim, me.anim_loaded = 0, 0xFF
    end
    local t = me.timer - 1
    me.timer = t
    if t > 0 then
        return
    end
    -- (Its panel is its thrower's still: the flight never sets it.)
    if not field.solid(me.panel_x, me.panel_y) then
        local q = me.pos
        battle.effect(Vec3.new(q.x, q.y, int.i32(q.z + SMOKE_Z)), effects.smoke)
        me.visible = false
    else
        local q = me.pos
        me.pos = Vec3.new(q.x, q.y, 0)
    end
    me:set_action(BURST)
end

-- Action 4 (0x02016090): its four hits, the shake and a sound; 20 ticks on a puff, and it goes.
local function burst(me: Object)
    if me.phase_init == 0 then
        local s = me.state :: State
        local f = me.facing
        for _, d in HITS[s.quarter + 1] do
            local h = battle.hitbox(me, {
                panel_x = me.future_panel_x + d[1] * f,
                panel_y = me.future_panel_y + d[2],
                element = me.element,
                z = 0,
                region = regions.single,
                target = collision.hits_navis,
                self_type = collision.attack,
                damage = me.damage,
                stamina = me.stamina,
                hit_mod = 3,
                status = statuses.confuse_480,
            })
            if h then
                h.run_while_dimmed = true
            end
        end
        battle.shake_camera(1, 0xA)
        battle.play_sound(SOUND_BURST)
        me.timer = BURST_TICKS
        me.phase_init = 4
    end
    local t = me.timer - 1
    me.timer = t
    if t > 0 then
        return
    end
    local q = me.pos
    battle.effect(Vec3.new(q.x, q.y, int.i32(q.z + PUFF_Z)), effects.puff)
    me:set_lifecycle("destroy")
end

local function tick(me: Object)
    local a = me.action
    if a == FLY then
        fly(me)
    elseif a == BURST then
        burst(me)
    else
        error(string.format("K-BOMB's bomb: action %#x reads past its table (0x020D80D4)", a))
    end
end

bomb.kind = new.kind {
    id = "k-bomb/bomb",
    pool = "attack",
    state = { quarter = "u8" },
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            -- 0x02015F60: its sprite and shadow, animation 1, shown, facing its way, on the panel under it; its target
            -- (kept as its future panel) three columns ahead, and its flight to the target's corner.
            local s = me.state :: State
            local sprite = me.sprite
            sprite:load(SPRITE)
            sprite.shadow = "ground"
            me.anim, me.anim_loaded = 1, 0xFF
            me.visible = true
            sprite:set_flip(bit32.bxor(me.alliance, me.flip))
            me:set_panel_from_coordinates()
            local f = me.facing
            me.future_panel_x, me.future_panel_y = me.panel_x + AHEAD * f, me.panel_y
            local x, y = field.center(me.future_panel_x, me.future_panel_y)
            local corner = CORNERS[s.quarter + 1]
            x = int.i32(x + corner[1] * 0x10000 * f)
            y = int.i32(y + corner[2] * 0x10000)
            -- (The DS's arc, 0x0208DB5C: lib/ds_arc.)
            me.vel = ds_arc.in_ticks(me.pos, x, y, 0, FLIGHT, GRAVITY, me.alliance)
            me:set_lifecycle("update")
            tick(me)
        elseif l == "update" then
            tick(me)
        else
            me:destroy()
            return
        end
        me:update_sprite()
    end,
}

-- 0x0201611C: a bomb of `thrower`'s at `pos` for quarter `quarter`, null, with its damage word; it runs while dimmed.
function bomb.spawn(thrower: Object, pos: Vec3, quarter: number): Object?
    local o = battle.spawn(bomb.kind, pos)
    if not o then
        return nil
    end
    (o.state :: State).quarter = quarter
    o.element = 0
    o.alliance, o.flip = thrower.alliance, thrower.flip
    o:set_damage_word(bit32.bor(thrower.damage, bit32.lshift(thrower.stamina, 16)))
    o.related1 = thrower
    o.run_while_dimmed = true
    return o
end

return bomb
