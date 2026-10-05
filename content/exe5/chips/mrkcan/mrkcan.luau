--!strict
-- EXE5's MarkCannons (action 0x4E, 0x080EF358: EXE5's own): the navi sends out a sight (effect object #0x44,
-- 0x080E5498: EXE6 keeps its code, `sub_80E5710`, which no EXE6 chip uses) that sweeps the field ahead in arcs; on the
-- first panel with an enemy body it locks on, paralyzing what stands there (a hit of no damage, element 5, status
-- 0x11), and the navi raises its cannon and fires at that panel 15 ticks later (hit modifier 3, the impact spark).
-- If the sight finds nothing (three arcs), or the navi's slot no longer holds it, the navi stands down. The three
-- chips differ in their cannon's palette.
--
-- The sight reports to the navi through the attack's marker word (AIAttackVars+0x30: its result byte, then the
-- panel found): 0 searching, 1 found, 2 nothing.

local attachment = require("../../objects/attachment")
local collision = require("../../rules/collision")
local regions = require("@exelib/regions")
local slot = require("@exelib/slot")
local sparks = require("../../lib/sparks")
local status = require("../../rules/status")
local trajectory = require("../../lib/trajectory")

local mrkcan = {}

-- ---- The sight ----------------------------------------------------------------------------------------------

export type SightState = {
    -- The slot of its owner that holds it (ExtraVars+0).
    slot: Slot,
    -- The center of its sweep, 16.16 (ExtraVars+0xC, +0x10), and how far the center moves forward each arc
    -- (XVelocity).
    center_x: number,
    center_y: number,
    advance: number,
    -- Its angle (ExtraVars+0x14), the arc's starting angle (+0x18), both 16.16 steps of 256 a turn, and the turn
    -- per tick (ZVelocity).
    angle: number,
    base: number,
    turn: number,
    -- The enemy bodies it looks for (ExtraVars+8).
    enemies: number,
}

local SIGHT_SPRITE = asset.sprite("mark-sight")
local LOCK_SOUND = asset.sound("mark")
-- The sweep: an arc of 16 ticks, 3 arcs; 120 pixels from the center, which starts 80 pixels behind the owner's
-- column on the middle row and moves 40 pixels on each arc; it starts at a quarter turn, turning 8 steps a tick,
-- the other way each arc.
local ARC_TICKS, ARCS = 0x10, 3
local RADIUS = 0x78
local START_BEHIND, ADVANCE = 0x50_0000, 0x28_0000
local START_ANGLE, TURN = 0x40_0000, -0x8_0000
-- The enemy bodies by side (0x080E5520).
local ENEMY_BODIES = { [0] = 0x04000000, [1] = 0x08000000 }
-- The lock-on: no damage, element 5, paralysis for 120 ticks (status 0x11), on the panel alone.
local LOCK_ELEMENT = 5

-- `sub_800E258`: the panel under a position.
local function panel_at(x: number, y: number): (number, number)
    return int.tdiv(int.asr(x, 16) + 0xA0, 0x28), int.tdiv(int.asr(y, 16) + 0x20, 0x18)
end

-- The owner's result: `result`, and the panel found.
local function report(me: Object, result: number, x: number?, y: number?)
    local owner = me.related1 :: Object
    if x and y then
        owner.marker = bit32.bor(result, bit32.lshift(x, 8), bit32.lshift(y, 16), bit32.band(owner.marker, 0xFF000000))
    else
        owner.marker = bit32.bor(bit32.band(owner.marker, 0xFFFFFF00), result)
    end
end

-- Nothing found: report it and go.
local function give_up(me: Object)
    report(me, 2)
    me:set_lifecycle("destroy")
end

-- `sub_80E58F4`: the arc's far point (the center 120 pixels ahead) is on the field.
local function in_reach(me: Object, s: SightState): boolean
    local x, y = panel_at(int.i32(me.facing * RADIUS * 0x10000 + s.center_x), s.center_y)
    return field.valid(x, y)
end

-- `sub_80E58D2`: the point `RADIUS` from the center at `angle` (the cosine toward the owner's front).
local function on_arc(me: Object, s: SightState, angle: number): (number, number)
    local cos, sin = trajectory.sine(angle + 64), trajectory.sine(angle)
    local x = s.center_x + int.mul32(int.mul32(cos, RADIUS) * 0x100, me.facing)
    local y = s.center_y + int.mul32(sin, RADIUS) * 0x100
    return int.i32(x), int.i32(y)
end

-- `sub_80E57BC`: sweep, an arc at a time; lock on to the first enemy body under it.
local function sweep(me: Object, s: SightState)
    local owner = me.related1
    if not owner or not slot.occupied(owner, s.slot) or not in_reach(me, s) then
        return give_up(me)
    end
    if me.phase_init == 0 then
        me.timer = ARC_TICKS
        me.phase_init = 4
    end
    local t = me.timer - 1
    me.timer = t
    if t < 0 then
        me.timer = ARC_TICKS
        local arcs = me.timer2 - 1
        me.timer2 = arcs
        if arcs <= 0 then
            return give_up(me)
        end
        s.center_x = int.i32(s.center_x + s.advance)
        s.turn = -s.turn
        local base = bit32.lshift(bit32.rshift(s.base, 16) + 0x80 + 0xFF, 16)
        s.base, s.angle = base, base
        return
    end
    s.angle = int.u32(s.angle + s.turn)
    local x, y = on_arc(me, s, bit32.band(bit32.rshift(s.angle, 16), 0xFF))
    local p = me.pos
    me.pos = Vec3.new(x, y, p.z)
    me:set_panel_from_coordinates()
    local flags = field.flags(me.panel_x, me.panel_y)
    if flags ~= 0 and bit32.btest(flags, s.enemies) then
        report(me, 1, me.panel_x, me.panel_y)
        me.action = 4
        me.phase, me.phase_init = 0, 0
    end
end

-- `sub_80E5872`: locked on: on the panel, paralyzing what stands there, until the owner lets it go.
local function locked(me: Object, s: SightState)
    local owner = me.related1
    if not owner or not slot.occupied(owner, s.slot) then
        me:set_lifecycle("destroy")
        return
    end
    if me.phase_init ~= 0 then
        return
    end
    me:set_coordinates_from_panel()
    me.anim, me.anim_loaded = 1, 0
    battle.play_sound(LOCK_SOUND)
    -- `sub_80E5910`.
    battle.hitbox(me, {
        panel_x = me.panel_x,
        panel_y = me.panel_y,
        element = LOCK_ELEMENT,
        region = regions.single,
        hit_effect = nil,
        target = collision.hits_navis,
        self_type = collision.attack,
        damage = 0,
        hit_mod = 0,
        status = status.paralyze_120,
    })
    me.phase_init = 4
end

-- `sub_80E5734`.
local function init(me: Object, s: SightState)
    local owner = me.related1 :: Object
    local dir = me.facing
    local cx, cy = field.center(owner.panel_x, 2)
    s.center_x = int.i32(cx - START_BEHIND * dir)
    s.center_y = cy
    local p = me.pos
    me.pos = Vec3.new(p.x, p.y, -0x10_0000)
    local sprite = me.sprite
    sprite:load(SIGHT_SPRITE)
    sprite.shadow = "with_sprite"
    me.anim, me.anim_loaded = 0, 0xFF
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    me.visible = true
    s.advance = ADVANCE * dir
    s.turn = TURN
    s.angle, s.base = START_ANGLE, START_ANGLE
    me.timer2 = ARCS
    s.enemies = ENEMY_BODIES[me.alliance]
    me:set_lifecycle("update")
end

mrkcan.sight = define.kind {
    id = "mrkcan/sight",
    pool = "effect",
    state = {
        slot = slot.TYPE,
        center_x = "i32",
        center_y = "i32",
        advance = "i32",
        angle = "u32",
        base = "u32",
        turn = "i32",
        enemies = "u32",
    },
    -- 0x080E5498 (`sub_80E5710`): by its state, then a sprite step.
    update = function(me: Object)
        local s = me.state :: SightState
        local l = me.lifecycle
        if l == "init" then
            init(me, s)
        end
        if me.lifecycle == "update" then
            local a = me.action
            if a == 0 then
                sweep(me, s)
            elseif a == 4 then
                locked(me, s)
            else
                error(string.format("MrkCan sight action %#x reads past its table", a))
            end
        elseif l ~= "init" then
            me:free()
            return
        end
        me:update_sprite()
    end,
}

-- `sub_80E58A8`: a sight for `owner`, held in its `which` slot.
local function send_sight(owner: Object, which: Slot): Object?
    local o = battle.spawn(mrkcan.sight, owner.pos)
    if o then
        o.alliance, o.flip = owner.alliance, owner.flip
        o.related1 = owner
        local s = o.state :: SightState
        s.slot = which
        o.run_while_dimmed = false
    end
    slot.fill(owner, which, o)
    return o
end

-- ---- The action ---------------------------------------------------------------------------------------------

export type State = {
    -- Ticks into the shot, then of the recovery (AIAttackVars+0x10).
    timer: number,
}

mrkcan.STATE = { timer = "u16" }

-- The phases, on the attack's step.
local AIM, FIRE, RECOVER = 0x0, 0x4, 0x8

local SHOT_SOUND = asset.sound("cannon")
-- The shot: on the panel found, 16 pixels up, hit modifier 3, the impact spark; at the 15th tick, 29 in all.
local FIRE_AT, FIRE_TICKS = 0x0F, 0x1D
-- The cannon's animation by the holder's AI index (0x080EF478: every player's 0; an AI index past it reads on).
local ANIM_ROWS = 0x18

-- The cannons (the attachment table's rows 0x2C to 0x2E, by the chip).
function mrkcan.cannon(palette: number): AttachmentLook
    return attachment.look { sprite = asset.sprite("mark-cannon"), palette = palette, attach_point = 18 }
end

-- 0x080EF460: the cannon's animation.
local function cannon_anim(me: Object): number
    if me.actor_type ~= "player" then
        return 0
    end
    if me.ai_index >= ANIM_ROWS then
        error(string.format("MrkCan: AI index %#x reads past 0x080EF478", me.ai_index))
    end
    return 0
end

-- A chip's MarkCannon.
function mrkcan.action(spec: { id: string, cannon: AttachmentLook }): Action
    -- 0x080EF378: send the sight, and wait for what it finds.
    local function aim(me: Object, s: State)
        if me.step_init == 0 then
            me.marker = 0
            send_sight(me, "related")
            me.step_init = 4
        end
        local result = bit32.band(me.marker, 0xFF)
        if result == 0 then
            return
        elseif result == 1 then
            me.step, me.step_init = FIRE, 0
            return
        end
        me:exit_attack()
    end

    -- 0x080EF3B8: raise the cannon and fire at the panel found.
    local function fire(me: Object, s: State)
        if me.step_init == 0 then
            me:set_animation(8)
            me:open_counter_window()
            s.timer = 0
            attachment.attach(me, spec.cannon, "overlay", { anim = cannon_anim(me) })
            me:set_status("using_action", true)
            me.step_init = 4
            return
        end
        if s.timer == FIRE_AT then
            battle.play_sound(SHOT_SOUND)
            local word = int.u32(me.attack_damage + me.hit_param * 0x10000 + me.extra)
            battle.hitbox(me, {
                panel_x = bit32.extract(me.marker, 8, 8),
                panel_y = bit32.extract(me.marker, 16, 8),
                element = 0,
                z = 0x10_0000,
                region = regions.single,
                hit_effect = sparks.impact,
                target = collision.hits_navis,
                self_type = collision.attack,
                damage = bit32.band(word, 0xFFFF),
                stamina = bit32.rshift(word, 16),
                hit_mod = 3,
            })
        end
        local t = s.timer + 1
        s.timer = t
        if t == FIRE_TICKS then
            me.step, me.step_init = RECOVER, 0
        end
    end

    -- 0x080EF434: the cannon and the sight go; 4 ticks, then back to idle.
    local function recover(me: Object, s: State)
        if me.step_init == 0 then
            me.related1 = nil
            me.overlay = nil
            me:set_animation(7)
            s.timer = 3
            me.step_init = 4
        end
        local t = s.timer - 1
        s.timer = t
        if t < 0 then
            me:exit_attack()
        end
    end

    return define.action {
        id = spec.id,
        state = mrkcan.STATE,
        args = spec,
        update = function(me: Object, s: State)
            local step = me.step
            if step == AIM then
                aim(me, s)
            elseif step == FIRE then
                fire(me, s)
            elseif step == RECOVER then
                recover(me, s)
            else
                error(string.format("MrkCan phase %#x reads past its phase table", step))
            end
        end,
    }
end

return mrkcan
